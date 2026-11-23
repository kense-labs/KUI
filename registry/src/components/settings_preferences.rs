//! Personal preferences screen for default models, language, appearance, chat tools, instructions, and memory.
//!
//! Renders inside [`AppShell`](crate::components::app_shell::AppShell) with the `Settings`
//! sidebar preset (`Preferences` active under `Personal`), providing workspace-filtered
//! chat and image model pickers (including `Auto` with an animated Model Wheel), language
//! and appearance controls, autonomous app-action and chat-tool toggles, pauseable custom
//! instructions (`1,500` characters per field), and per-item or bulk memory management.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProEmptyCta, ProPalette, pro_btn, pro_ease, pro_eid, pro_empty_state,
            pro_fold, pro_mark_selected, pro_pill_indicator, pro_presence, pro_selection_sensor,
            pro_skeleton_bar, pro_spring_f32, pro_switch, pro_toast_banner, pro_view_transition,
            tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        flyout::TriggerBounds,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const MAX_INSTRUCTION_CHARS: usize = 1_500;
const WHEEL_STEP_MS: f32 = 2_200.0;
const WHEEL_GLIDE_MS: f32 = 560.0;

const CHECK_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.1"/>"#;
const EXPAND_SVG: &str = r#"<path d="M16.5 3h4.5v4.5M21 3l-6 6M7.5 21H3v-4.5M3 21l6-6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"/>"#;
const COLLAPSE_SVG: &str = r#"<path d="M15 9h6M15 9V3M15 9l6-6M9 15H3M9 15v6M9 15l-6 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"/>"#;
const INFO_CIRCLE_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12 16V12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12.125 8.25H12M12.25 8.25C12.25 8.11193 12.1381 8 12 8C11.8619 8 11.75 8.11193 11.75 8.25C11.75 8.38807 11.8619 8.5 12 8.5C12.1381 8.5 12.25 8.38807 12.25 8.25Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const SYSTEM_MONITOR_SVG: &str = r#"<path d="M14 21H16M14 21C13.1716 21 12.5 20.3284 12.5 19.5V17L12 17M14 21H10M10 21H8M10 21C10.8284 21 11.5 20.3284 11.5 19.5V17L12 17M12 17V21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M16 3H8C5.17157 3 3.75736 3 2.87868 3.87868C2 4.75736 2 6.17157 2 9V11C2 13.8284 2 15.2426 2.87868 16.1213C3.75736 17 5.17157 17 8 17H16C18.8284 17 20.2426 17 21.1213 16.1213C22 15.2426 22 13.8284 22 11V9C22 6.17157 22 4.75736 21.1213 3.87868C20.2426 3 18.8284 3 16 3Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;
const SUN_SVG: &str = r#"<path d="M17 12C17 14.7614 14.7614 17 12 17C9.23858 17 7 14.7614 7 12C7 9.23858 9.23858 7 12 7C14.7614 7 17 9.23858 17 12Z" stroke="currentColor" stroke-width="1.9"/><path d="M12 2V3.5M12 20.5V22M19.0708 19.0713L18.0101 18.0106M5.98926 5.98926L4.9286 4.9286M22 12H20.5M3.5 12H2M19.0713 4.92871L18.0106 5.98937M5.98975 18.0107L4.92909 19.0714" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"/>"#;
const MOON_SVG: &str = r#"<path d="M21.5 14.0784C20.3003 14.7189 18.9301 15.0821 17.4751 15.0821C12.7491 15.0821 8.91792 11.2509 8.91792 6.52485C8.91792 5.06986 9.28105 3.69968 9.92163 2.5C5.66765 3.49698 2.5 7.31513 2.5 11.8731C2.5 17.1899 6.8101 21.5 12.1269 21.5C16.6849 21.5 20.503 18.3324 21.5 14.0784Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const TOOL_WEB_SEARCH_SVG: &str = r#"<path d="M22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"/><path d="M20 5.69899C19.0653 5.76636 17.8681 6.12824 17.0379 7.20277C15.5385 9.14361 14.039 9.30556 13.0394 8.65861C11.5399 7.6882 12.8 6.11636 11.0401 5.26215C9.89313 4.70542 9.73321 3.19045 10.3716 2" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"/><path d="M2 11C2.7625 11.6621 3.83046 12.2682 5.08874 12.2682C7.68843 12.2682 8.20837 12.7649 8.20837 14.7518C8.20837 16.7387 8.20837 16.7387 8.72831 18.2288C9.06651 19.1981 9.18472 20.1674 8.5106 21" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"/><path d="M19.8988 19.9288L22 22M21.1083 17.0459C21.1083 19.2805 19.2932 21.0919 17.0541 21.0919C14.8151 21.0919 13 19.2805 13 17.0459C13 14.8114 14.8151 13 17.0541 13C19.2932 13 21.1083 14.8114 21.1083 17.0459Z" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"/>"#;
const TOOL_IMAGE_GEN_SVG: &str = r#"<circle cx="7.5" cy="7.5" r="1.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" stroke="currentColor" stroke-width="1.9"/><path d="M5 21C9.37246 15.775 14.2741 8.88406 21.4975 13.5424" stroke="currentColor" stroke-width="1.9"/>"#;
const TOOL_MEMORY_SVG: &str = r#"<path d="M15.1449 5.20762C14.5031 4.46785 13.5562 4 12.5 4C11.0033 4 9.72595 4.93951 9.22564 6.26097C8.85144 6.09327 8.43661 6 8 6C6.34315 6 5 7.34315 5 9C5 9.01673 5.00014 9.03343 5.00041 9.05009M15.1449 5.20762C15.5725 5.07274 16.0278 5 16.5 5C18.9853 5 21 7.01472 21 9.5C21 10.1296 20.8707 10.729 20.6372 11.273M15.1449 5.20762C13.7981 5.63239 12.7249 6.67345 12.2561 8C12.1435 8.31841 12.0658 8.65327 12.0275 9M9.05001 14C9.28164 15.1411 10.2905 16 11.5 16C11.7548 16 11.8823 16 11.9998 16.014C12.5855 16.0835 13.1107 16.4081 13.4348 16.9009C13.4999 16.9997 13.5569 17.1137 13.6708 17.3416L15 20M9.05001 14C9.01722 13.8384 9 13.6712 9 13.5C9 12.6822 9.39267 11.9561 9.99976 11.5M9.05001 14H4.5C3.11929 14 2 12.8807 2 11.5C2 10.1193 3.11929 9 4.5 9C4.67138 9 4.83873 9.01724 5.00041 9.05009M20.6372 11.273C20.2961 11.0985 19.9095 11 19.5 11C18.2905 11 17.2816 11.8589 17.05 13M20.6372 11.273C21.4462 11.687 22 12.5288 22 13.5C22 14.8807 20.8807 16 19.5 16C18.2904 16 17.4531 17.2082 17.8778 18.3408L18.5 20M5.00041 9.05009C5.01267 9.7988 5.2992 10.4808 5.76389 11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const FLAG_EN_SVG: &str = r##"<g transform="translate(0 3)"><rect width="20" height="14" fill="#012169"/><path d="M0 0 20 14M20 0 0 14" stroke="#ffffff" stroke-width="2.8"/><path d="M0 0 20 14M20 0 0 14" stroke="#c8102e" stroke-width="1"/><path d="M10 0v14M0 7h20" stroke="#ffffff" stroke-width="4.2"/><path d="M10 0v14M0 7h20" stroke="#c8102e" stroke-width="2.4"/></g>"##;
const FLAG_DE_SVG: &str = r##"<g transform="translate(0 3)"><rect width="20" height="4.67" fill="#111111"/><rect y="4.67" width="20" height="4.67" fill="#dd0000"/><rect y="9.33" width="20" height="4.67" fill="#ffce00"/></g>"##;
const FLAG_FR_SVG: &str = r##"<g transform="translate(0 3)"><rect width="6.67" height="14" fill="#002395"/><rect x="6.67" width="6.67" height="14" fill="#ffffff"/><rect x="13.33" width="6.67" height="14" fill="#ed2939"/></g>"##;
const FLAG_ES_SVG: &str = r##"<g transform="translate(0 3)"><rect width="20" height="3.5" fill="#aa151b"/><rect y="3.5" width="20" height="7" fill="#f1bf00"/><rect y="10.5" width="20" height="3.5" fill="#aa151b"/></g>"##;
const FLAG_JA_SVG: &str = r##"<g transform="translate(0 3)"><rect width="20" height="14" fill="#ffffff"/><circle cx="10" cy="7" r="4.2" fill="#bc002d"/></g>"##;
const FLAG_ZH_SVG: &str = r##"<g transform="translate(0 3)"><rect width="20" height="14" fill="#de2910"/><circle cx="5.2" cy="5.2" r="2.2" fill="#ffde00"/><circle cx="9.5" cy="2.6" r="0.8" fill="#ffde00"/><circle cx="11.2" cy="4.3" r="0.8" fill="#ffde00"/><circle cx="11.0" cy="6.6" r="0.8" fill="#ffde00"/><circle cx="9.3" cy="8.2" r="0.8" fill="#ffde00"/></g>"##;

const LANGUAGE_OPTIONS: &[(&str, &str)] = &[
    ("English", FLAG_EN_SVG),
    ("Deutsch", FLAG_DE_SVG),
    ("Français", FLAG_FR_SVG),
    ("Español", FLAG_ES_SVG),
    ("日本語", FLAG_JA_SVG),
    ("简体中文", FLAG_ZH_SVG),
];

const DEFAULT_ABOUT_TEXT: &str = "I lead product design at Acme, a 40-person B2B SaaS company. I mostly ask about UX copy, research synthesis and planning.";

/// Personal theme preference (`System` follows OS appearance).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppearanceChoice {
    /// Follow the operating system's light or dark mode.
    #[default]
    System,
    /// Always use light mode.
    Light,
    /// Always use dark mode.
    Dark,
}

impl AppearanceChoice {
    /// All appearance choices in segmented-control display order.
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// Canonical English label for this appearance choice.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    /// Localized label matching the active UI language.
    pub fn label_tr(self, cx: &App) -> &'static str {
        match self {
            Self::System => tr(cx, "System", "跟随系统"),
            Self::Light => tr(cx, "Light", "浅色"),
            Self::Dark => tr(cx, "Dark", "深色"),
        }
    }

    fn icon_svg(self) -> &'static str {
        match self {
            Self::System => SYSTEM_MONITOR_SVG,
            Self::Light => SUN_SVG,
            Self::Dark => MOON_SVG,
        }
    }
}

/// Toggle states for personal chat tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatToolsConfig {
    /// Allow the assistant to search the web for fresh facts.
    pub web_search: bool,
    /// Allow the assistant to generate images on request.
    pub image_generation: bool,
    /// Allow the assistant to retain memories across conversations.
    pub memory: bool,
}

impl Default for ChatToolsConfig {
    fn default() -> Self {
        Self {
            web_search: true,
            image_generation: true,
            memory: false,
        }
    }
}

/// Custom system instructions prepended to every user chat message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomInstructions {
    /// Whether custom instructions are currently active (`false` pauses without clearing text).
    pub enabled: bool,
    /// Background context about the person (`<= 1,500` characters).
    pub about: SharedString,
    /// Tone and formatting guidance (`<= 1,500` characters).
    pub style: SharedString,
}

impl Default for CustomInstructions {
    fn default() -> Self {
        Self {
            enabled: true,
            about: DEFAULT_ABOUT_TEXT.into(),
            style: "".into(),
        }
    }
}

impl CustomInstructions {
    /// Return whether both instruction fields fit within the 1,500-character limit.
    pub fn is_valid(&self) -> bool {
        self.about.chars().count() <= MAX_INSTRUCTION_CHARS
            && self.style.chars().count() <= MAX_INSTRUCTION_CHARS
    }
}

/// Single personal memory item retained from past chats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedMemory {
    /// Stable identifier for forgetting a single memory row.
    pub id: SharedString,
    /// Remembered fact shown in the memory list.
    pub text: SharedString,
    /// Relative timestamp caption (e.g. `"Saved 2 days ago"`).
    pub saved_at: SharedString,
}

impl SavedMemory {
    /// Construct a saved memory item.
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        saved_at: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            saved_at: saved_at.into(),
        }
    }
}

/// Workspace-level model entry used to populate chat and image model pickers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceModelOption {
    /// Unique model slug.
    pub id: SharedString,
    /// Human-readable model name shown in the combobox.
    pub label: SharedString,
    /// Provider artwork key (`"auto"`, `"claude"`, `"openai"`, `"gemini"`, `"deepseek"`, `"flux"`).
    pub provider: SharedString,
    /// `true` for image-generation models, `false` for chat models.
    pub is_image: bool,
    /// Whether the workspace administrator has enabled this model.
    pub enabled: bool,
}

impl WorkspaceModelOption {
    /// Construct an enabled chat model option.
    pub fn chat(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        provider: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            provider: provider.into(),
            is_image: false,
            enabled: true,
        }
    }

    /// Construct an enabled image model option.
    pub fn image(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        provider: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            provider: provider.into(),
            is_image: true,
            enabled: true,
        }
    }

    /// Set whether this model is enabled in the workspace.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Default workspace model catalog matching the Pro settings suite.
pub fn default_workspace_models() -> Vec<WorkspaceModelOption> {
    vec![
        WorkspaceModelOption::chat("auto", "Auto", "auto"),
        WorkspaceModelOption::chat("claude-sonnet-5-5", "Claude Sonnet 5.5", "claude"),
        WorkspaceModelOption::chat("claude-opus-4-6", "Claude Opus 4.6", "claude"),
        WorkspaceModelOption::chat("gpt-5-4", "GPT-5.4", "openai"),
        WorkspaceModelOption::chat("gemini-2-5-pro", "Gemini 2.5 Pro", "gemini"),
        WorkspaceModelOption::chat("deepseek-r2", "DeepSeek R2", "deepseek"),
        WorkspaceModelOption::image("gpt-image-2", "GPT Image 2", "openai"),
        WorkspaceModelOption::image("imagen-4", "Imagen 4", "gemini"),
        WorkspaceModelOption::image("flux-1-1-pro", "Flux 1.1 Pro", "flux"),
    ]
}

/// Default sample memories shown on [`SettingsPreferencesScreen`].
pub fn default_sample_memories() -> Vec<SavedMemory> {
    vec![
        SavedMemory::new(
            "mem-1",
            "Prefers answers in English, but German examples for customer-facing copy",
            "Saved 2 days ago",
        ),
        SavedMemory::new(
            "mem-2",
            "Works in Figma and Linear; the team ships every second Thursday",
            "Saved last week",
        ),
        SavedMemory::new(
            "mem-3",
            "Likes short bullet points over long paragraphs",
            "Saved 3 weeks ago",
        ),
    ]
}

/// Complete set of personal preferences edited by [`SettingsPreferencesScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferenceValues {
    /// Default chat model label (`"Claude Sonnet 5.5"` or `"Auto"`).
    pub default_model: SharedString,
    /// Default image generation model label (`"GPT Image 2"`).
    pub image_model: SharedString,
    /// Selected interface and assistant language (`"English"`).
    pub language: SharedString,
    /// Selected color appearance (`System`, `Light`, or `Dark`).
    pub appearance: AppearanceChoice,
    /// Allow the assistant to discover connected app actions without explicit `@` tags.
    pub auto_app_actions: bool,
    /// Enabled personal chat tools.
    pub chat_tools: ChatToolsConfig,
    /// Custom instructions configuration.
    pub instructions: CustomInstructions,
    /// Saved personal memories.
    pub memories: Vec<SavedMemory>,
}

impl Default for PreferenceValues {
    fn default() -> Self {
        Self {
            default_model: "Claude Sonnet 5.5".into(),
            image_model: "GPT Image 2".into(),
            language: "English".into(),
            appearance: AppearanceChoice::System,
            auto_app_actions: false,
            chat_tools: ChatToolsConfig::default(),
            instructions: CustomInstructions::default(),
            memories: default_sample_memories(),
        }
    }
}

/// Account identity displayed at the bottom of the settings sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPreferencesUser {
    /// Full display name of the signed-in user.
    pub name: SharedString,
    /// Email address shown in the account menu.
    pub email: SharedString,
}

impl Default for SettingsPreferencesUser {
    fn default() -> Self {
        Self {
            name: "Alex Morgan".into(),
            email: "alex@acme.com".into(),
        }
    }
}

impl SettingsPreferencesUser {
    /// Construct a sidebar user identity with `name` and `email`.
    pub fn new(name: impl Into<SharedString>, email: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Events emitted by [`SettingsPreferencesScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsPreferencesEvent {
    /// Emitted with the committed [`PreferenceValues`] whenever preferences are saved.
    Save(PreferenceValues),
    /// Emitted immediately when the appearance segmented control changes.
    AppearanceChanged(AppearanceChoice),
    /// Emitted with the forgotten memory `id` when an individual memory is removed.
    MemoryForgotten(SharedString),
    /// Emitted when all saved memories are cleared.
    MemoriesCleared,
    /// Emitted when `"Start a chat"` is clicked in the empty memories state.
    StartChat,
    /// Emitted when a settings sidebar item or `"Back to app"` is activated.
    Navigate(SharedString),
}

impl EventEmitter<SettingsPreferencesEvent> for SettingsPreferencesScreen {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenPicker {
    DefaultModel,
    ImageModel,
    Language,
}

fn settings_nav_groups(cx: &App) -> Vec<ShellNavGroup> {
    vec![
        ShellNavGroup::named(
            tr(cx, "Personal", "个人"),
            vec![
                ShellNavItem::new("profile", tr(cx, "Profile", "个人资料"), ProIcon::User),
                ShellNavItem::new(
                    "preferences",
                    tr(cx, "Preferences", "偏好设置"),
                    ProIcon::Sliders,
                ),
                ShellNavItem::new(
                    "notifications",
                    tr(cx, "Notifications", "通知"),
                    ProIcon::Bell,
                ),
                ShellNavItem::new("privacy", tr(cx, "Privacy", "隐私"), ProIcon::Shield),
            ],
        ),
        ShellNavGroup::named(
            tr(cx, "Workspace", "工作区"),
            vec![
                ShellNavItem::new("general", tr(cx, "General", "常规"), ProIcon::Settings),
                ShellNavItem::new("members", tr(cx, "Members", "成员与团队"), ProIcon::Users),
                ShellNavItem::new("models", tr(cx, "Models", "模型"), ProIcon::Building),
                ShellNavItem::new("security", tr(cx, "Security", "安全"), ProIcon::KeyLock),
                ShellNavItem::new(
                    "billing",
                    tr(cx, "Billing", "账单与订阅"),
                    ProIcon::CreditCard,
                ),
            ],
        ),
    ]
}

fn format_char_count(n: usize) -> String {
    if n < 1_000 {
        n.to_string()
    } else {
        format!("{},{:03}", n / 1_000, n % 1_000)
    }
}

fn flag_svg_for_language(lang: &str) -> &'static str {
    LANGUAGE_OPTIONS
        .iter()
        .find(|(name, _)| *name == lang)
        .map(|(_, svg)| *svg)
        .unwrap_or(FLAG_EN_SVG)
}

fn render_language_flag(lang: &str) -> Div {
    let svg = flag_svg_for_language(lang);
    div()
        .w(px(16.))
        .h(px(11.))
        .flex_none()
        .rounded(px(2.5))
        .overflow_hidden()
        .border_1()
        .border_color(hsla(0., 0., 0., 0.14))
        .flex()
        .items_center()
        .justify_center()
        .child(pro_svg("0 0 20 20", svg, 16., rgb(0xffffff).into()))
}

fn render_model_mark(
    provider: &str,
    model_label: &str,
    side: f32,
    p: &ProPalette,
    cx: &App,
) -> AnyElement {
    let lower_prov = provider.trim().to_ascii_lowercase();
    let lower_label = model_label.trim().to_ascii_lowercase();
    if lower_prov == "auto" || lower_label == "auto" {
        return pro_icon(ProIcon::Sparkles, side, p.strong);
    }
    if let Some(mark) = pro_app_artwork(provider, side, cx) {
        return mark;
    }
    if let Some(mark) = pro_app_artwork(model_label, side, cx) {
        return mark;
    }
    pro_icon(ProIcon::Bolt, side, p.muted)
}

/// Personal preferences screen (`settings-preferences`).
pub struct SettingsPreferencesScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    workspace: SharedString,
    user: SettingsPreferencesUser,
    providers: Vec<WorkspaceModelOption>,
    saved: PreferenceValues,
    draft: PreferenceValues,
    open_picker: Option<OpenPicker>,
    last_picker: Option<OpenPicker>,
    picker_highlight: usize,
    default_model_bounds: TriggerBounds,
    image_model_bounds: TriggerBounds,
    language_bounds: TriggerBounds,
    about_expanded: bool,
    style_expanded: bool,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
    wheel_started_at: Instant,
    page_width: Rc<Cell<f32>>,
    about_editor: Option<Entity<Editor>>,
    style_editor: Option<Entity<Editor>>,
}

impl Focusable for SettingsPreferencesScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SettingsPreferencesScreen {
    /// Construct a new [`SettingsPreferencesScreen`] with sample personal preferences.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let saved = PreferenceValues::default();
        let draft = saved.clone();
        let workspace: SharedString = "Acme".into();
        let user = SettingsPreferencesUser::default();
        let ws_clone = workspace.clone();
        let user_name = user.name.clone();
        let user_email = user.email.clone();
        let this = cx.entity();

        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Settings, shell_cx)
                .with_groups(settings_nav_groups(shell_cx))
                .with_workspace(ws_clone)
                .with_user(user_name, user_email)
                .with_active("Preferences")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsPreferencesEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsPreferencesEvent::Navigate("Back to app".into()));
            }
            _ => {}
        })
        .detach();

        let now = Instant::now();
        Self {
            id: ElementId::Name(id_str),
            focus,
            shell,
            standalone: false,
            loading: false,
            empty: false,
            workspace,
            user,
            providers: default_workspace_models(),
            saved,
            draft,
            open_picker: None,
            last_picker: None,
            picker_highlight: 0,
            default_model_bounds: TriggerBounds::default(),
            image_model_bounds: TriggerBounds::default(),
            language_bounds: TriggerBounds::default(),
            about_expanded: false,
            style_expanded: false,
            toast: None,
            started_at: now,
            wheel_started_at: now,
            page_width: Rc::new(Cell::new(720.)),
            about_editor: None,
            style_editor: None,
        }
    }

    /// Override the initial [`PreferenceValues`].
    pub fn with_preferences(mut self, preferences: PreferenceValues) -> Self {
        self.saved = preferences.clone();
        self.draft = preferences;
        self
    }

    /// Override the workspace model catalog; disabled models are excluded from pickers.
    pub fn with_providers(mut self, providers: Vec<WorkspaceModelOption>) -> Self {
        self.providers = providers;
        self
    }

    /// Override the workspace name shown in the sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws: SharedString = workspace.into();
        self.workspace = ws.clone();
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_workspace(ws);
        });
        self
    }

    /// Override the account displayed at the bottom of the settings sidebar.
    pub fn with_user(mut self, user: SettingsPreferencesUser, cx: &mut Context<Self>) -> Self {
        let name = user.name.clone();
        let email = user.email.clone();
        self.user = user;
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_user(name, email);
        });
        self
    }

    /// Start in skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Start in empty state (no custom instructions text and empty memories list).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.saved.instructions.about = "".into();
            self.saved.instructions.style = "".into();
            self.saved.memories.clear();
            self.draft = self.saved.clone();
        }
        self
    }

    /// Render only the page column without the outer [`AppShell`] frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Return the last committed preferences.
    pub fn preferences(&self) -> &PreferenceValues {
        &self.saved
    }

    /// Return the workspace name shown in the sidebar.
    pub fn workspace(&self) -> &str {
        self.workspace.as_ref()
    }

    /// Return the sidebar account summary.
    pub fn user(&self) -> &SettingsPreferencesUser {
        &self.user
    }

    /// Return the working draft preferences including unsaved instruction edits.
    pub fn draft(&self) -> &PreferenceValues {
        &self.draft
    }

    /// Return whether working draft fields differ from committed preferences.
    pub fn is_dirty(&self) -> bool {
        self.draft != self.saved
    }

    /// Return whether skeleton loading placeholders are active.
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

    /// Return whether empty mode is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, w: Option<&Window>, cx: &mut Context<Self>) {
        self.empty = empty;
        if empty {
            self.saved.instructions.about = "".into();
            self.saved.instructions.style = "".into();
            self.saved.memories.clear();
            self.draft = self.saved.clone();
            if let Some(win) = w {
                if let Some(ed) = &self.about_editor {
                    ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
                }
                if let Some(ed) = &self.style_editor {
                    ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
                }
            }
        }
        cx.notify();
    }

    /// Return enabled chat models from the workspace catalog.
    pub fn available_chat_models(&self) -> Vec<WorkspaceModelOption> {
        self.providers
            .iter()
            .filter(|m| !m.is_image && m.enabled)
            .cloned()
            .collect()
    }

    /// Return enabled image models from the workspace catalog.
    pub fn available_image_models(&self) -> Vec<WorkspaceModelOption> {
        self.providers
            .iter()
            .filter(|m| m.is_image && m.enabled)
            .cloned()
            .collect()
    }

    /// Pick a default chat model immediately (only if enabled in workspace) and emit [`SettingsPreferencesEvent::Save`].
    pub fn select_default_model(
        &mut self,
        model_label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let label: SharedString = model_label.into();
        self.open_picker = None;
        let allowed = self
            .available_chat_models()
            .iter()
            .any(|m| m.label == label);
        if !allowed {
            cx.notify();
            return false;
        }
        if self.draft.default_model != label {
            self.draft.default_model = label.clone();
            self.saved.default_model = label.clone();
            if label.as_ref() == "Auto" {
                self.wheel_started_at = Instant::now();
            }
            self.show_toast(tr(cx, "Default model updated", "默认模型已更新"), cx);
            cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        }
        cx.notify();
        true
    }

    /// Switch the default chat model to `"Auto"` and reset the Model Wheel phase.
    pub fn switch_to_auto(&mut self, cx: &mut Context<Self>) {
        let _ = self.select_default_model("Auto", cx);
    }

    /// Pick an image model immediately (only if enabled in workspace) and emit [`SettingsPreferencesEvent::Save`].
    pub fn select_image_model(
        &mut self,
        model_label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let label: SharedString = model_label.into();
        self.open_picker = None;
        let allowed = self
            .available_image_models()
            .iter()
            .any(|m| m.label == label);
        if !allowed {
            cx.notify();
            return false;
        }
        if self.draft.image_model != label {
            self.draft.image_model = label.clone();
            self.saved.image_model = label;
            self.show_toast(tr(cx, "Image model updated", "图像模型已更新"), cx);
            cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        }
        cx.notify();
        true
    }

    /// Pick a display and assistant language immediately and emit [`SettingsPreferencesEvent::Save`].
    pub fn select_language(&mut self, language: impl Into<SharedString>, cx: &mut Context<Self>) {
        let lang: SharedString = language.into();
        self.open_picker = None;
        if self.draft.language != lang {
            self.draft.language = lang.clone();
            self.saved.language = lang;
            self.show_toast(tr(cx, "Language updated", "语言偏好已更新"), cx);
            cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        }
        cx.notify();
    }

    /// Select an appearance option immediately, emitting [`SettingsPreferencesEvent::AppearanceChanged`] and [`SettingsPreferencesEvent::Save`].
    pub fn select_appearance(&mut self, choice: AppearanceChoice, cx: &mut Context<Self>) {
        if self.draft.appearance != choice {
            self.draft.appearance = choice;
            self.saved.appearance = choice;
            cx.emit(SettingsPreferencesEvent::AppearanceChanged(choice));
            cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
            cx.notify();
        }
    }

    /// Toggle autonomous app action discovery and emit [`SettingsPreferencesEvent::Save`].
    pub fn toggle_auto_app_actions(&mut self, cx: &mut Context<Self>) {
        let next = !self.draft.auto_app_actions;
        self.draft.auto_app_actions = next;
        self.saved.auto_app_actions = next;
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
    }

    /// Toggle the Web search chat tool and emit [`SettingsPreferencesEvent::Save`].
    pub fn toggle_web_search(&mut self, cx: &mut Context<Self>) {
        let next = !self.draft.chat_tools.web_search;
        self.draft.chat_tools.web_search = next;
        self.saved.chat_tools.web_search = next;
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
    }

    /// Toggle the Image generation chat tool and emit [`SettingsPreferencesEvent::Save`].
    pub fn toggle_image_generation(&mut self, cx: &mut Context<Self>) {
        let next = !self.draft.chat_tools.image_generation;
        self.draft.chat_tools.image_generation = next;
        self.saved.chat_tools.image_generation = next;
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
    }

    /// Toggle the Memory chat tool and emit [`SettingsPreferencesEvent::Save`].
    pub fn toggle_memory_tool(&mut self, cx: &mut Context<Self>) {
        let next = !self.draft.chat_tools.memory;
        self.draft.chat_tools.memory = next;
        self.saved.chat_tools.memory = next;
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
    }

    /// Toggle custom instructions without discarding text; disables editors while paused.
    pub fn toggle_instructions_enabled(&mut self, w: Option<&Window>, cx: &mut Context<Self>) {
        let next = !self.draft.instructions.enabled;
        self.draft.instructions.enabled = next;
        self.saved.instructions.enabled = next;
        if let Some(win) = w {
            if let Some(ed) = &self.about_editor {
                ed.update(cx, |e, e_cx| {
                    e.disabled = !next;
                    e.set_read_only(!next, win, e_cx);
                });
            }
            if let Some(ed) = &self.style_editor {
                ed.update(cx, |e, e_cx| {
                    e.disabled = !next;
                    e.set_read_only(!next, win, e_cx);
                });
            }
        }
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
    }

    /// Update draft custom instruction text fields programmatically.
    pub fn set_draft_instructions(
        &mut self,
        about: impl Into<SharedString>,
        style: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.draft.instructions.about = about.into();
        self.draft.instructions.style = style.into();
        cx.notify();
    }

    /// Commit working draft instruction changes if within the 1,500-character limit.
    pub fn commit_save(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.draft.instructions.is_valid() || self.draft == self.saved {
            return false;
        }
        self.saved = self.draft.clone();
        self.show_toast(tr(cx, "Preferences saved", "偏好设置已保存"), cx);
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
        true
    }

    /// Revert unsaved custom instruction edits back to the committed values.
    pub fn discard_changes(&mut self, w: Option<&Window>, cx: &mut Context<Self>) {
        self.draft = self.saved.clone();
        if let Some(win) = w {
            if let Some(ed) = &self.about_editor {
                let v = self.saved.instructions.about.to_string();
                ed.update(cx, |e, e_cx| e.set_value(v, win, e_cx));
            }
            if let Some(ed) = &self.style_editor {
                let v = self.saved.instructions.style.to_string();
                ed.update(cx, |e, e_cx| e.set_value(v, win, e_cx));
            }
        }
        cx.notify();
    }

    /// Forget a single saved memory by `id`, emitting [`SettingsPreferencesEvent::MemoryForgotten`] and [`SettingsPreferencesEvent::Save`].
    pub fn forget_memory(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let before = self.draft.memories.len();
        self.draft.memories.retain(|m| m.id.as_ref() != id);
        if self.draft.memories.len() == before {
            return false;
        }
        self.saved.memories = self.draft.memories.clone();
        let forgotten_id: SharedString = id.to_owned().into();
        self.show_toast(tr(cx, "Memory forgotten", "已移除该条记忆"), cx);
        cx.emit(SettingsPreferencesEvent::MemoryForgotten(forgotten_id));
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
        true
    }

    /// Clear all saved memories at once, emitting [`SettingsPreferencesEvent::MemoriesCleared`] and [`SettingsPreferencesEvent::Save`].
    pub fn clear_memories(&mut self, cx: &mut Context<Self>) {
        if self.draft.memories.is_empty() {
            return;
        }
        self.draft.memories.clear();
        self.saved.memories.clear();
        self.show_toast(tr(cx, "All memories cleared", "已清空全部记忆"), cx);
        cx.emit(SettingsPreferencesEvent::MemoriesCleared);
        cx.emit(SettingsPreferencesEvent::Save(self.saved.clone()));
        cx.notify();
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn toggle_picker(&mut self, which: OpenPicker, cx: &mut Context<Self>) {
        if self.open_picker == Some(which) {
            self.open_picker = None;
        } else {
            self.open_picker = Some(which);
            self.last_picker = Some(which);
            self.picker_highlight = match which {
                OpenPicker::DefaultModel => self
                    .available_chat_models()
                    .iter()
                    .position(|m| m.label == self.draft.default_model)
                    .unwrap_or(0),
                OpenPicker::ImageModel => self
                    .available_image_models()
                    .iter()
                    .position(|m| m.label == self.draft.image_model)
                    .unwrap_or(0),
                OpenPicker::Language => LANGUAGE_OPTIONS
                    .iter()
                    .position(|(l, _)| *l == self.draft.language.as_ref())
                    .unwrap_or(0),
            };
        }
        cx.notify();
    }

    fn picker_len(&self, which: OpenPicker) -> usize {
        match which {
            OpenPicker::DefaultModel => self.available_chat_models().len(),
            OpenPicker::ImageModel => self.available_image_models().len(),
            OpenPicker::Language => LANGUAGE_OPTIONS.len(),
        }
    }

    fn confirm_picker_choice(&mut self, which: OpenPicker, idx: usize, cx: &mut Context<Self>) {
        match which {
            OpenPicker::DefaultModel => {
                if let Some(m) = self.available_chat_models().get(idx) {
                    let _ = self.select_default_model(m.label.clone(), cx);
                }
            }
            OpenPicker::ImageModel => {
                if let Some(m) = self.available_image_models().get(idx) {
                    let _ = self.select_image_model(m.label.clone(), cx);
                }
            }
            OpenPicker::Language => {
                if let Some((lang, _)) = LANGUAGE_OPTIONS.get(idx) {
                    self.select_language(*lang, cx);
                }
            }
        }
    }

    fn provider_for_model(&self, label: &str, is_image: bool) -> String {
        self.providers
            .iter()
            .find(|m| m.is_image == is_image && m.label.as_ref() == label)
            .map(|m| m.provider.to_string())
            .unwrap_or_else(|| {
                if label.eq_ignore_ascii_case("auto") {
                    "auto".to_owned()
                } else if is_image {
                    "openai".to_owned()
                } else {
                    "claude".to_owned()
                }
            })
    }

    fn wheel_models(&self) -> Vec<(SharedString, SharedString)> {
        let mut list: Vec<(SharedString, SharedString)> = self
            .available_chat_models()
            .into_iter()
            .filter(|m| m.label.as_ref() != "Auto")
            .map(|m| (m.label, m.provider))
            .collect();
        let fallbacks = [
            ("Claude Sonnet 5.5", "claude"),
            ("GPT-5.4", "openai"),
            ("Gemini 2.5 Pro", "gemini"),
            ("Claude Opus 4.6", "claude"),
            ("DeepSeek R2", "deepseek"),
        ];
        for (lbl, prov) in fallbacks {
            if list.len() >= 5 {
                break;
            }
            if !list.iter().any(|(existing, _)| existing.as_ref() == lbl) {
                list.push((lbl.into(), prov.into()));
            }
        }
        list
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let paused = !self.draft.instructions.enabled;
        if self.about_editor.is_none() {
            let label: SharedString = tr(cx, "About you", "关于你").into();
            let ph: SharedString = tr(
                cx,
                "What should the model know about you to answer better?",
                "告诉模型你的职责、行业背景或常处理的问题，以便提供更贴合的回答。",
            )
            .into();
            let initial = self.draft.instructions.about.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Multi, w, e_cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e.set_command_submit(true);
                if !initial.is_empty() {
                    e.set_value(initial, w, e_cx);
                }
                e.disabled = paused;
                e.set_read_only(paused, w, e_cx);
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    if this.draft.instructions.enabled {
                        this.draft.instructions.about = ed_ref.read(cx).value().to_owned().into();
                        cx.notify();
                    }
                }
                EditEvent::Submitted if this.draft.instructions.enabled => {
                    this.draft.instructions.about = ed_ref.read(cx).value().to_owned().into();
                    this.commit_save(cx);
                }
                _ => {}
            })
            .detach();
            self.about_editor = Some(ed);
        }

        if self.style_editor.is_none() {
            let label: SharedString = tr(cx, "How it should answer", "回答风格与要求").into();
            let ph: SharedString = tr(
                cx,
                "E.g. Short and direct. Bullet points for lists. Say when you're unsure.",
                "例如：简短直接。列表使用要点符号。不确定时请明确说明。",
            )
            .into();
            let initial = self.draft.instructions.style.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Multi, w, e_cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e.set_command_submit(true);
                if !initial.is_empty() {
                    e.set_value(initial, w, e_cx);
                }
                e.disabled = paused;
                e.set_read_only(paused, w, e_cx);
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    if this.draft.instructions.enabled {
                        this.draft.instructions.style = ed_ref.read(cx).value().to_owned().into();
                        cx.notify();
                    }
                }
                EditEvent::Submitted if this.draft.instructions.enabled => {
                    this.draft.instructions.style = ed_ref.read(cx).value().to_owned().into();
                    this.commit_save(cx);
                }
                _ => {}
            })
            .detach();
            self.style_editor = Some(ed);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_picker_trigger(
        &self,
        id_suffix: &'static str,
        aria: SharedString,
        leading: AnyElement,
        value_label: SharedString,
        width: Option<f32>,
        which: OpenPicker,
        bounds: TriggerBounds,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let is_open = self.open_picker == Some(which);
        let input_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let border_col = if is_open {
            p.ink.opacity(0.32)
        } else {
            p.line_strong
        };
        let hover_border = p.ink.opacity(0.24);
        let focus = p.focus;
        let chevron_icon = if is_open {
            ProIcon::ChevronUp
        } else {
            ProIcon::ChevronDown
        };

        let mut trigger = div()
            .id(pro_eid(&self.id, id_suffix))
            .role(Role::ComboBox)
            .aria_label(aria)
            .aria_expanded(is_open)
            .tab_index(0)
            .relative()
            .h(px(32.))
            .pl(px(10.))
            .pr(px(8.))
            .rounded(px(7.))
            .bg(input_bg)
            .border_1()
            .border_color(border_col)
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_pointer()
            .hover(move |s| s.border_color(hover_border))
            .focus_visible(move |s| s.border_1().border_color(focus))
            .child(bounds.measure())
            .child(
                div()
                    .size(px(16.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(leading),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(value_label),
            )
            .child(pro_icon(chevron_icon, 14., p.muted));

        trigger = match width {
            Some(w_px) => trigger.w(px(w_px)).flex_none(),
            None => trigger.w_full(),
        };

        activate(trigger, cx, move |this, _, cx| {
            this.toggle_picker(which, cx);
        })
    }

    fn render_open_picker_overlay(
        &self,
        w: &Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let is_open = self.open_picker.is_some();
        let which = self.open_picker.or(self.last_picker)?;
        let (bounds, picker_key) = match which {
            OpenPicker::DefaultModel => (&self.default_model_bounds, "default-model"),
            OpenPicker::ImageModel => (&self.image_model_bounds, "image-model"),
            OpenPicker::Language => (&self.language_bounds, "language"),
        };
        let menu_bg: Hsla = if p.dark {
            rgb(0x1f1f22).into()
        } else {
            rgb(0xffffff).into()
        };

        let items_count = self.picker_len(which);
        let est_h = (items_count as f32 * 33.0 + 10.0).min(300.0);
        let rect = bounds.get();
        let (pos, anchor_side) = bounds.place(w.viewport_size(), px(est_h), px(4.));
        let menu_w = f32::from(rect.size.width).max(188.0);
        let anchor_copy = bounds.clone();

        let mut list = div()
            .id(pro_eid(&self.id, "picker-menu"))
            .role(Role::ListBox)
            .max_h(px(296.))
            .overflow_y_scroll()
            .p(px(4.))
            .flex()
            .flex_col()
            .gap(px(1.));

        match which {
            OpenPicker::DefaultModel => {
                for (idx, opt) in self.available_chat_models().into_iter().enumerate() {
                    let selected = self.draft.default_model == opt.label;
                    let highlighted = self.picker_highlight == idx;
                    let row_bg = if highlighted {
                        p.hover
                    } else {
                        transparent_black()
                    };
                    let hover_bg = p.hover;
                    let label_clone = opt.label.clone();
                    let mark = render_model_mark(&opt.provider, &opt.label, 14., p, cx);
                    list = list.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("chat-model-opt-{idx}")))
                            .role(Role::ListBoxOption)
                            .aria_label(opt.label.clone())
                            .aria_selected(selected)
                            .min_h(px(32.))
                            .px(px(8.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .bg(row_bg)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .child(
                                div()
                                    .size(px(16.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(mark),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .font_weight(if selected {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(p.ink)
                                    .child(opt.label),
                            )
                            .when(selected, |d| {
                                d.child(pro_svg("0 0 24 24", CHECK_SVG, 14., p.ink))
                            }),
                        cx,
                        move |this, _, cx| {
                            let _ = this.select_default_model(label_clone.clone(), cx);
                        },
                    ));
                }
            }
            OpenPicker::ImageModel => {
                for (idx, opt) in self.available_image_models().into_iter().enumerate() {
                    let selected = self.draft.image_model == opt.label;
                    let highlighted = self.picker_highlight == idx;
                    let row_bg = if highlighted {
                        p.hover
                    } else {
                        transparent_black()
                    };
                    let hover_bg = p.hover;
                    let label_clone = opt.label.clone();
                    let mark = render_model_mark(&opt.provider, &opt.label, 14., p, cx);
                    list = list.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("img-model-opt-{idx}")))
                            .role(Role::ListBoxOption)
                            .aria_label(opt.label.clone())
                            .aria_selected(selected)
                            .min_h(px(32.))
                            .px(px(8.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .bg(row_bg)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .child(
                                div()
                                    .size(px(16.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(mark),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .font_weight(if selected {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(p.ink)
                                    .child(opt.label),
                            )
                            .when(selected, |d| {
                                d.child(pro_svg("0 0 24 24", CHECK_SVG, 14., p.ink))
                            }),
                        cx,
                        move |this, _, cx| {
                            let _ = this.select_image_model(label_clone.clone(), cx);
                        },
                    ));
                }
            }
            OpenPicker::Language => {
                for (idx, (lang, _)) in LANGUAGE_OPTIONS.iter().enumerate() {
                    let selected = self.draft.language.as_ref() == *lang;
                    let highlighted = self.picker_highlight == idx;
                    let row_bg = if highlighted {
                        p.hover
                    } else {
                        transparent_black()
                    };
                    let hover_bg = p.hover;
                    let lang_val = *lang;
                    list = list.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("lang-opt-{idx}")))
                            .role(Role::ListBoxOption)
                            .aria_label(*lang)
                            .aria_selected(selected)
                            .min_h(px(32.))
                            .px(px(8.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .bg(row_bg)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .child(render_language_flag(lang))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .font_weight(if selected {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(p.ink)
                                    .child(*lang),
                            )
                            .when(selected, |d| {
                                d.child(pro_svg("0 0 24 24", CHECK_SVG, 14., p.ink))
                            }),
                        cx,
                        move |this, _, cx| {
                            this.select_language(lang_val, cx);
                        },
                    ));
                }
            }
        }

        let panel = div()
            .w(px(menu_w))
            .rounded(px(10.))
            .bg(menu_bg)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .when(is_open, |d| {
                d.occlude().on_mouse_down_out(cx.listener(
                    move |this, ev: &MouseDownEvent, _, cx| {
                        if !anchor_copy.get().contains(&ev.position) {
                            this.open_picker = None;
                            cx.notify();
                        }
                    },
                ))
            })
            .child(pro_view_transition(
                pro_eid(&self.id, "picker-menu-vtx"),
                picker_key,
                list,
            ))
            .into_any_element();

        Some(pro_presence(
            pro_eid(&self.id, "picker-menu-pres"),
            is_open,
            0.18,
            0.14,
            move |amount| {
                deferred(
                    anchored()
                        .anchor(anchor_side)
                        .position(pos)
                        .snap_to_window_with_margin(px(10.))
                        .child(
                            div()
                                .relative()
                                .top(px(-4. * (1. - amount)))
                                .opacity(amount)
                                .child(panel),
                        ),
                )
                .with_priority(3)
                .into_any_element()
            },
        ))
    }

    fn render_model_wheel(&self, narrow: bool, reduced: bool, p: &ProPalette, cx: &App) -> Div {
        let models = self.wheel_models();
        let n = models.len().max(1);
        let (base_idx, frac) = if reduced {
            (0usize, 0.0f32)
        } else {
            let elapsed = self.wheel_started_at.elapsed().as_secs_f32() * 1_000.0;
            let step = (elapsed / WHEEL_STEP_MS).floor() as usize;
            let rem = elapsed % WHEEL_STEP_MS;
            let hold = WHEEL_STEP_MS - WHEEL_GLIDE_MS;
            let f = if rem <= hold {
                0.0
            } else {
                pro_ease((rem - hold) / WHEEL_GLIDE_MS)
            };
            (step % n, f)
        };

        let pill_bg: Hsla = if p.dark { rgb(0x26262a).into() } else { p.card };

        let mut drum = div()
            .relative()
            .h(px(176.))
            .overflow_hidden()
            .when(narrow, |d| d.w_full())
            .when(!narrow, |d| d.w(px(240.)).flex_none())
            .child(
                div()
                    .absolute()
                    .left(px(10.))
                    .right(px(10.))
                    .top(px(69.))
                    .h(px(38.))
                    .rounded(px(9.))
                    .bg(pill_bg)
                    .border_1()
                    .border_color(p.line_strong),
            );

        for offset in -2i32..=2i32 {
            let rel = offset as f32 - frac;
            let angle = (rel * 0.62).clamp(-1.45, 1.45);
            let cos_a = angle.cos().max(0.0);
            if cos_a <= 0.05 {
                continue;
            }
            let idx = (base_idx as i32 + offset).rem_euclid(n as i32) as usize;
            let (label, provider) = &models[idx];
            let y = 69.0 + rel * 34.0;
            let inset_x = 12.0 + (1.0 - cos_a) * 16.0;
            let alpha = cos_a.powf(1.45).clamp(0.14, 1.0);
            let is_center = rel.abs() < 0.45;
            let fg = if is_center {
                p.strong
            } else {
                p.muted.opacity(alpha)
            };
            let mark =
                render_model_mark(provider, label, if is_center { 15.5 } else { 14. }, p, cx);

            drum = drum.child(
                div()
                    .absolute()
                    .top(px(y))
                    .left(px(inset_x))
                    .right(px(inset_x))
                    .h(px(38.))
                    .px(px(12.))
                    .opacity(alpha)
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .child(mark)
                    .child(
                        div()
                            .truncate()
                            .text_size(px(if is_center { 13.5 } else { 12.5 }))
                            .font_weight(if is_center {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(fg)
                            .child(label.clone()),
                    ),
            );
        }

        drum
    }

    fn render_appearance_segmented(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let track_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.045)
        };
        let pill_bg: Hsla = if p.dark {
            rgb(0x2c2c2f).into()
        } else {
            rgb(0xffffff).into()
        };

        let sensor = pro_selection_sensor();
        let mut row = div()
            .relative()
            .h(px(28.))
            .p(px(2.))
            .rounded(px(8.))
            .bg(track_bg)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "appearance-pill"),
                self.draft.appearance.label(),
                sensor.clone(),
                6.,
                pill_bg,
                Some(p.line),
                true,
            ));

        for choice in AppearanceChoice::ALL {
            let active = self.draft.appearance == choice;
            let label = choice.label_tr(cx);
            let fg = if active { p.ink } else { p.text };
            let icon_col = if active { p.ink } else { p.muted };
            let hover_bg = p.hover;
            let focus = p.focus;

            row = row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("appearance-{}", choice.label())))
                    .role(Role::RadioButton)
                    .aria_label(label)
                    .aria_toggled(active.into())
                    .tab_index(0)
                    .relative()
                    .h(px(24.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .when(active, |d| d.child(pro_mark_selected(&sensor)))
                    .when(!active, |d| d.hover(move |s| s.bg(hover_bg)))
                    .focus_visible(move |s| s.border_1().border_color(focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(5.5))
                    .cursor_pointer()
                    .child(pro_svg("0 0 24 24", choice.icon_svg(), 14., icon_col))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(if active {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(fg)
                            .child(label),
                    ),
                cx,
                move |this, _, cx| {
                    this.select_appearance(choice, cx);
                },
            ));
        }

        row
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tool_tile(
        &self,
        id_suffix: &'static str,
        icon_svg: &'static str,
        title: SharedString,
        caption: SharedString,
        checked: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
        on_toggle: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static,
    ) -> Stateful<Div> {
        let check_fill: Hsla = if p.dark { p.blue } else { rgb(0x171717).into() };
        let icon_color = if checked { p.strong } else { p.muted };
        let hover_border = p.line_strong;
        let focus = p.focus;

        let check_box = if checked {
            div()
                .size(px(16.))
                .flex_none()
                .rounded(px(5.))
                .bg(check_fill)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_svg("0 0 24 24", CHECK_SVG, 11., rgb(0xffffff).into()))
        } else {
            div()
                .size(px(16.))
                .flex_none()
                .rounded(px(5.))
                .border_1()
                .border_color(p.line_strong)
        };

        activate(
            div()
                .id(pro_eid(&self.id, id_suffix))
                .role(Role::CheckBox)
                .aria_label(title.clone())
                .aria_toggled(checked.into())
                .tab_index(0)
                .flex_1()
                .min_w_0()
                .pt(px(12.))
                .pb(px(14.))
                .px(px(14.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(2.))
                .cursor_pointer()
                .hover(move |s| s.border_color(hover_border))
                .focus_visible(move |s| s.border_1().border_color(focus))
                .child(
                    div()
                        .mb(px(8.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_svg("0 0 24 24", icon_svg, 16., icon_color))
                        .child(check_box),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(caption),
                ),
            cx,
            on_toggle,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_instruction_field(
        &self,
        id_suffix: &'static str,
        label: SharedString,
        editor: Option<&Entity<Editor>>,
        char_count: usize,
        expanded: bool,
        box_h: f32,
        paused: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
        on_expand: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static,
    ) -> Div {
        let over = char_count > MAX_INSTRUCTION_CHARS;
        let input_bg: Hsla = if paused {
            p.subtle
        } else if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let border_col = if over {
            p.red.opacity(0.65)
        } else {
            p.line_strong
        };
        let count_col = if over {
            p.red_ink
        } else if char_count >= MAX_INSTRUCTION_CHARS {
            p.muted
        } else {
            p.faint
        };
        let expand_aria = if expanded {
            tr(cx, "Less room", "收起输入框")
        } else {
            tr(cx, "More room", "展开输入框")
        };
        let expand_svg = if expanded { COLLAPSE_SVG } else { EXPAND_SVG };
        let hover_wash = p.hover;
        let focus = p.focus;

        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(label),
            )
            .child(
                div()
                    .relative()
                    .h(px(box_h))
                    .py(px(7.))
                    .pl(px(10.))
                    .pr(px(36.))
                    .rounded(px(8.))
                    .bg(input_bg)
                    .border_1()
                    .border_color(border_col)
                    .flex()
                    .items_start()
                    .when_some(editor, |d, ed| d.child(div().size_full().child(ed.clone())))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, id_suffix))
                            .role(Role::Button)
                            .aria_label(expand_aria)
                            .tab_index(0)
                            .absolute()
                            .top(px(4.))
                            .right(px(4.))
                            .size(px(26.))
                            .rounded(px(5.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_wash))
                            .focus_visible(move |s| s.border_1().border_color(focus))
                            .child(pro_svg("0 0 24 24", expand_svg, 15., p.muted)),
                        cx,
                        on_expand,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.red_ink)
                            .when(over, |d| {
                                d.child(tr(
                                    cx,
                                    "Keep instructions under 1,500 characters.",
                                    "请将自定义指令控制在 1,500 个字符以内。",
                                ))
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(count_col)
                            .child(format!(
                                "{} / {}",
                                format_char_count(char_count),
                                format_char_count(MAX_INSTRUCTION_CHARS)
                            )),
                    ),
            )
    }

    fn render_loading_skeleton(&self, narrow: bool, phase: f32, p: &ProPalette, cx: &App) -> Div {
        let model_w = if narrow { 180. } else { 200. };
        let lang_w = if narrow { 150. } else { 160. };

        div()
            .w_full()
            .max_w(px(760.))
            .mx_auto()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(24.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Preferences", "偏好设置")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Make the assistant work the way you do.",
                        "让助手按照你的工作习惯协作。",
                    )),
            )
            .child(
                div()
                    .mt(px(36.))
                    .child(pro_skeleton_bar(64., 14., 4., phase, cx)),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(96., 12., 4., phase + 0.04, cx))
                                    .child(pro_skeleton_bar(188., 10., 4., phase + 0.08, cx)),
                            )
                            .child(pro_skeleton_bar(model_w, 32., 7., phase + 0.12, cx)),
                    )
                    .child(
                        div()
                            .ml(px(16.))
                            .mr(px(12.))
                            .mb(px(12.))
                            .p(px(14.))
                            .rounded(px(10.))
                            .bg(p.page)
                            .flex()
                            .items_center()
                            .gap(px(14.))
                            .child(pro_skeleton_bar(32., 32., 9., phase + 0.16, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(210., 12., 4., phase + 0.2, cx))
                                    .child(pro_skeleton_bar(280., 10., 4., phase + 0.24, cx)),
                            )
                            .child(pro_skeleton_bar(92., 24., 6., phase + 0.28, cx)),
                    )
                    .child(
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(86., 12., 4., phase + 0.32, cx))
                                    .child(pro_skeleton_bar(164., 10., 4., phase + 0.36, cx)),
                            )
                            .child(pro_skeleton_bar(model_w, 32., 7., phase + 0.4, cx)),
                    ),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(72., 12., 4., phase + 0.44, cx))
                                    .child(pro_skeleton_bar(182., 10., 4., phase + 0.48, cx)),
                            )
                            .child(pro_skeleton_bar(lang_w, 32., 7., phase + 0.52, cx)),
                    )
                    .child(
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(84., 12., 4., phase + 0.56, cx))
                                    .child(pro_skeleton_bar(148., 10., 4., phase + 0.6, cx)),
                            )
                            .child(pro_skeleton_bar(186., 28., 8., phase + 0.64, cx)),
                    ),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .min_h(px(60.))
                    .py(px(10.))
                    .pl(px(16.))
                    .pr(px(12.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(24.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(186., 12., 4., phase + 0.68, cx))
                            .child(pro_skeleton_bar(270., 10., 4., phase + 0.72, cx)),
                    )
                    .child(pro_skeleton_bar(34., 20., 10., phase + 0.76, cx)),
            )
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(pro_skeleton_bar(78., 14., 4., phase + 0.8, cx))
                    .child(pro_skeleton_bar(264., 12., 4., phase + 0.84, cx)),
            )
            .child({
                let mut grid = div().mt(px(12.)).flex().gap(px(8.));
                if narrow {
                    grid = grid.flex_col();
                }
                for i in 0..3 {
                    let off = phase + 0.88 + i as f32 * 0.06;
                    grid = grid.child(
                        div()
                            .flex_1()
                            .p(px(14.))
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(pro_skeleton_bar(16., 16., 4., off, cx))
                                    .child(pro_skeleton_bar(16., 16., 5., off + 0.02, cx)),
                            )
                            .child(pro_skeleton_bar(96., 12., 4., off + 0.03, cx))
                            .child(pro_skeleton_bar(148., 10., 4., off + 0.04, cx)),
                    );
                }
                grid
            })
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(pro_skeleton_bar(136., 14., 4., phase + 1.08, cx))
                    .child(pro_skeleton_bar(310., 12., 4., phase + 1.12, cx)),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(pro_skeleton_bar(142., 12., 4., phase + 1.16, cx))
                            .child(pro_skeleton_bar(34., 20., 10., phase + 1.2, cx)),
                    )
                    .child(pro_skeleton_bar(340., 72., 8., phase + 1.24, cx))
                    .child(pro_skeleton_bar(340., 72., 8., phase + 1.28, cx)),
            )
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(68., 14., 4., phase + 1.32, cx))
                            .child(pro_skeleton_bar(290., 12., 4., phase + 1.36, cx)),
                    )
                    .child(pro_skeleton_bar(68., 24., 6., phase + 1.4, cx)),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children((0..3).map(|i| {
                        let off = phase + 1.44 + i as f32 * 0.06;
                        div()
                            .min_h(px(52.))
                            .px(px(16.))
                            .py(px(10.))
                            .when(i > 0, |d| d.border_t_1().border_color(p.line))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(290., 12., 4., off, cx))
                                    .child(pro_skeleton_bar(96., 10., 4., off + 0.03, cx)),
                            )
                            .child(pro_skeleton_bar(24., 24., 6., off + 0.04, cx))
                    })),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let is_auto = self.draft.default_model.as_ref() == "Auto";
        let phase = if reduced {
            0.25
        } else {
            (self.started_at.elapsed().as_secs_f32() * 0.9).fract()
        };

        if (self.loading || is_auto) && !reduced {
            w.request_animation_frame();
        }

        if let Some((_, ts)) = &self.toast {
            if ts.elapsed().as_millis() > 2_450 {
                self.toast = None;
            } else if !reduced {
                w.request_animation_frame();
            }
        }

        let measured_w = self.page_width.get();
        let narrow = measured_w < 600.;
        let pad_x = if measured_w < 640. { 20. } else { 32. };

        let width_cell = self.page_width.clone();
        let probe = canvas(
            move |bounds, _, _| {
                let w_px = f32::from(bounds.size.width);
                if w_px > 10. {
                    width_cell.set(w_px);
                }
            },
            |_, _, _, _| {},
        )
        .w_full()
        .h(px(0.));

        if self.loading {
            return div()
                .id(pro_eid(&self.id, "page-loading"))
                .size_full()
                .bg(p.panel)
                .overflow_hidden()
                .child(
                    div()
                        .id(pro_eid(&self.id, "scroll-loading"))
                        .size_full()
                        .overflow_y_scroll()
                        .pt(px(46.))
                        .pb(px(56.))
                        .px(px(pad_x))
                        .child(probe)
                        .child(self.render_loading_skeleton(narrow, phase, &p, cx)),
                );
        }

        let dirty = self.is_dirty();
        let valid = self.draft.instructions.is_valid();
        let paused = !self.draft.instructions.enabled;
        let about_count = self.draft.instructions.about.chars().count();
        let style_count = self.draft.instructions.style.chars().count();
        let about_box_h = pro_spring_f32(
            pro_eid(&self.id, "about-box-h"),
            if self.about_expanded { 148. } else { 72. },
            0.24,
            w,
            cx,
        );
        let style_box_h = pro_spring_f32(
            pro_eid(&self.id, "style-box-h"),
            if self.style_expanded { 148. } else { 72. },
            0.24,
            w,
            cx,
        );
        let instr_alpha = pro_spring_f32(
            pro_eid(&self.id, "instr-alpha"),
            if paused { 0.45 } else { 1.0 },
            0.20,
            w,
            cx,
        );
        let scroll_pb = pro_spring_f32(
            pro_eid(&self.id, "scroll-pb"),
            if dirty { 104. } else { 56. },
            0.22,
            w,
            cx,
        );

        let chat_provider = self.provider_for_model(&self.draft.default_model, false);
        let chat_mark = render_model_mark(&chat_provider, &self.draft.default_model, 15., &p, cx);
        let chat_trigger = self.render_picker_trigger(
            "default-model-trigger",
            tr(cx, "Default model", "默认模型").into(),
            chat_mark,
            self.draft.default_model.clone(),
            if narrow { None } else { Some(200.) },
            OpenPicker::DefaultModel,
            self.default_model_bounds.clone(),
            &p,
            cx,
        );

        let img_provider = self.provider_for_model(&self.draft.image_model, true);
        let img_mark = render_model_mark(&img_provider, &self.draft.image_model, 15., &p, cx);
        let img_trigger = self.render_picker_trigger(
            "image-model-trigger",
            tr(cx, "Image model", "图像模型").into(),
            img_mark,
            self.draft.image_model.clone(),
            if narrow { None } else { Some(200.) },
            OpenPicker::ImageModel,
            self.image_model_bounds.clone(),
            &p,
            cx,
        );

        let lang_flag = render_language_flag(&self.draft.language).into_any_element();
        let lang_trigger = self.render_picker_trigger(
            "language-trigger",
            tr(cx, "Language", "语言").into(),
            lang_flag,
            self.draft.language.clone(),
            if narrow { None } else { Some(160.) },
            OpenPicker::Language,
            self.language_bounds.clone(),
            &p,
            cx,
        );

        let auto_banner = if is_auto {
            let mut box_el = div()
                .ml(px(if narrow { 14. } else { 16. }))
                .mr(px(if narrow { 14. } else { 12. }))
                .mb(px(12.))
                .px(px(20.))
                .py(px(18.))
                .rounded(px(10.))
                .bg(p.page)
                .flex();
            if narrow {
                box_el = box_el.flex_col().items_stretch().gap(px(16.));
            } else {
                box_el = box_el.items_center().gap(px(32.));
            }
            box_el
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "How Auto works", "自动模式工作原理")),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .mb(px(10.))
                                .max_w(px(400.))
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Auto picks the model and how long it thinks for each message – the strongest one for hard questions, a fast one for quick replies.",
                                    "自动模式会为每条消息选择最合适的模型与思考时长——复杂问题调用最强模型，快速问答即时响应。",
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_size(px(12.5))
                                        .line_height(px(18.))
                                        .text_color(p.ink)
                                        .child(pro_svg("0 0 24 24", CHECK_SVG, 14., p.muted))
                                        .child(tr(
                                            cx,
                                            "The best answer for what it costs",
                                            "平衡回答质量与额度成本",
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_size(px(12.5))
                                        .line_height(px(18.))
                                        .text_color(p.ink)
                                        .child(pro_svg("0 0 24 24", CHECK_SVG, 14., p.muted))
                                        .child(tr(
                                            cx,
                                            "Only models you've turned on, hosted in the EU first",
                                            "仅使用工作区已启用的模型，优先合规区域托管",
                                        )),
                                ),
                        ),
                )
                .child(self.render_model_wheel(narrow, reduced, &p, cx))
        } else {
            let switch_btn = pro_btn(
                pro_eid(&self.id, "switch-to-auto"),
                tr(cx, "Switch to Auto", "切换为自动"),
                None,
                ProBtnStyle::Primary,
                true,
                cx,
                |this, _, cx| {
                    this.switch_to_auto(cx);
                },
            );
            let mut nudge = div()
                .ml(px(if narrow { 14. } else { 16. }))
                .mr(px(if narrow { 14. } else { 12. }))
                .mb(px(12.))
                .py(px(14.))
                .pl(px(16.))
                .pr(px(14.))
                .rounded(px(10.))
                .bg(p.page)
                .flex()
                .gap(px(14.));
            if narrow {
                nudge = nudge.flex_col().items_start();
            } else {
                nudge = nudge.items_center();
            }
            nudge
                .child(
                    div()
                        .size(px(32.))
                        .flex_none()
                        .rounded(px(9.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Sparkles, 16., p.strong)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(
                                    cx,
                                    "Get more out of your plan with Auto",
                                    "开启自动模式，让套餐额度更耐用",
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "It picks the right model for each message, so quick questions don't use up your credits.",
                                    "为每条消息自动挑选合适模型，简单提问不再消耗高阶模型额度。",
                                )),
                        ),
                )
                .child(switch_btn)
        };

        let appearance_seg = self.render_appearance_segmented(&p, cx);

        let app_actions_switch = pro_switch(
            pro_eid(&self.id, "auto-app-actions-switch"),
            tr(cx, "Find app actions on its own", "自动发现应用操作"),
            self.draft.auto_app_actions,
            cx,
            |this, _, cx| {
                this.toggle_auto_app_actions(cx);
            },
        );

        let web_tile = self.render_tool_tile(
            "tool-web-search",
            TOOL_WEB_SEARCH_SVG,
            tr(cx, "Web search", "网页搜索").into(),
            tr(
                cx,
                "Looks things up when it needs fresh facts",
                "需要最新信息时自动检索网页",
            )
            .into(),
            self.draft.chat_tools.web_search,
            &p,
            cx,
            |this, _, cx| this.toggle_web_search(cx),
        );
        let img_tile = self.render_tool_tile(
            "tool-image-gen",
            TOOL_IMAGE_GEN_SVG,
            tr(cx, "Image generation", "图像生成").into(),
            tr(
                cx,
                "Draws pictures when you ask for one",
                "按你的描述绘制配图与插画",
            )
            .into(),
            self.draft.chat_tools.image_generation,
            &p,
            cx,
            |this, _, cx| this.toggle_image_generation(cx),
        );
        let mem_tile = self.render_tool_tile(
            "tool-memory",
            TOOL_MEMORY_SVG,
            tr(cx, "Memory", "跨对话记忆").into(),
            tr(
                cx,
                "Remembers things you tell it across chats",
                "在不同对话间记住你交代的偏好与事实",
            )
            .into(),
            self.draft.chat_tools.memory,
            &p,
            cx,
            |this, _, cx| this.toggle_memory_tool(cx),
        );

        let mut tools_grid = div().mt(px(12.)).flex().gap(px(8.));
        if narrow {
            tools_grid = tools_grid.flex_col();
        }
        tools_grid = tools_grid.child(web_tile).child(img_tile).child(mem_tile);

        let instructions_switch = pro_switch(
            pro_eid(&self.id, "use-instructions-switch"),
            tr(cx, "Use my instructions", "启用我的自定义指令"),
            self.draft.instructions.enabled,
            cx,
            |this, w, cx| {
                this.toggle_instructions_enabled(Some(w), cx);
            },
        );

        let about_ed_clone = self.about_editor.clone();
        let style_ed_clone = self.style_editor.clone();
        let about_field = self.render_instruction_field(
            "expand-about",
            tr(cx, "About you", "关于你").into(),
            about_ed_clone.as_ref(),
            about_count,
            self.about_expanded,
            about_box_h,
            paused,
            &p,
            cx,
            |this, _, cx| {
                this.about_expanded = !this.about_expanded;
                cx.notify();
            },
        );
        let style_field = self.render_instruction_field(
            "expand-style",
            tr(cx, "How it should answer", "回答风格与要求").into(),
            style_ed_clone.as_ref(),
            style_count,
            self.style_expanded,
            style_box_h,
            paused,
            &p,
            cx,
            |this, _, cx| {
                this.style_expanded = !this.style_expanded;
                cx.notify();
            },
        );

        let has_memories = !self.empty && !self.draft.memories.is_empty();
        let memory_section = {
            let mut head = div().mt(px(36.)).flex().justify_between().gap(px(24.));
            if narrow {
                head = head.flex_col().items_start().gap(px(10.));
            } else {
                head = head.items_end();
            }
            head = head
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
                                .child(tr(cx, "Memory", "记忆库")),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "What the assistant has kept from your chats. Only you can see it.",
                                    "助手从过往对话中保留的偏好记录，仅你自己可见。",
                                )),
                        ),
                )
                .when(has_memories, |d| {
                    d.child(pro_btn(
                        pro_eid(&self.id, "clear-all-memories"),
                        tr(cx, "Clear all", "全部清空"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.clear_memories(cx);
                        },
                    ))
                });

            let mut sec = div().flex().flex_col().child(head);

            let mem_off_note = div()
                .pt(px(12.))
                .child(
                    div()
                        .py(px(10.))
                        .px(px(12.))
                        .rounded(px(10.))
                        .bg(p.page)
                        .flex()
                        .items_start()
                        .gap(px(8.))
                        .child(
                            div()
                                .mt(px(1.5))
                                .flex_none()
                                .child(pro_svg("0 0 24 24", INFO_CIRCLE_SVG, 15., p.faint)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Memory is off, so nothing new is kept. Turn it on under Chat tools.",
                                    "跨对话记忆当前已关闭，不会保存新内容。可在上方「对话工具」中开启。",
                                )),
                        ),
                )
                .into_any_element();
            sec = sec.child(pro_fold(
                pro_eid(&self.id, "mem-off-note-fold"),
                !self.draft.chat_tools.memory && has_memories,
                54.,
                0.,
                move || mem_off_note,
            ));

            let mem_state_key = if has_memories {
                "list"
            } else if self.draft.chat_tools.memory {
                "empty-on"
            } else {
                "empty-off"
            };
            let mem_body: AnyElement = if has_memories {
                let mut card = div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col();

                for (idx, mem) in self.draft.memories.clone().into_iter().enumerate() {
                    let mem_id = mem.id.clone();
                    let aria_forget: SharedString =
                        format!("{}: {}", tr(cx, "Forget", "遗忘"), mem.text).into();
                    let hover_bg = p.hover;
                    let focus = p.focus;

                    card = card.child(
                        div()
                            .id(pro_eid(&self.id, format!("mem-row-{}", mem.id)))
                            .min_h(px(52.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                            .flex()
                            .items_center()
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
                                            .text_size(px(13.))
                                            .line_height(px(20.))
                                            .text_color(p.ink)
                                            .child(mem.text),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .child(mem.saved_at),
                                    ),
                            )
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("forget-{}", mem.id)))
                                    .role(Role::Button)
                                    .aria_label(aria_forget)
                                    .tab_index(0)
                                    .size(px(28.))
                                    .flex_none()
                                    .rounded(px(8.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(hover_bg))
                                    .focus_visible(move |s| s.border_1().border_color(focus))
                                    .child(pro_icon(ProIcon::Close, 14., p.muted)),
                                cx,
                                move |this, _, cx| {
                                    this.forget_memory(&mem_id, cx);
                                },
                            )),
                    );
                }

                card.into_any_element()
            } else {
                let mem_on = self.draft.chat_tools.memory;
                let caption = if mem_on {
                    tr(
                        cx,
                        "Facts and preferences the assistant keeps from your chats will show up here.",
                        "助手从对话中记住的事实与偏好会显示在这里。",
                    )
                } else {
                    tr(
                        cx,
                        "Turn on memory so the assistant can keep helpful details across chats.",
                        "开启跨对话记忆后，助手会在不同对话间记住有用的信息。",
                    )
                };
                let cta_label: SharedString = if mem_on {
                    tr(cx, "Start a chat", "开始新对话").into()
                } else {
                    tr(cx, "Turn on memory", "开启记忆").into()
                };
                let cta: ProEmptyCta<Self> = (
                    pro_eid(&self.id, "empty-memories-cta"),
                    cta_label,
                    Box::new(move |this, _, cx| {
                        if mem_on {
                            cx.emit(SettingsPreferencesEvent::StartChat);
                            cx.emit(SettingsPreferencesEvent::Navigate("New chat".into()));
                        } else {
                            this.toggle_memory_tool(cx);
                        }
                    }),
                );

                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .child(pro_empty_state(
                        ProIcon::Memory,
                        tr(cx, "No memories yet", "暂无已存记忆"),
                        caption,
                        Some(cta),
                        cx,
                    ))
                    .into_any_element()
            };

            sec.child(pro_view_transition(
                pro_eid(&self.id, "memories-vtx"),
                mem_state_key,
                mem_body,
            ))
        };

        let picker_overlay = self.render_open_picker_overlay(w, &p, cx);

        let mut root = div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                let key = ev.keystroke.key.as_str();
                if let Some(which) = this.open_picker {
                    let len = this.picker_len(which);
                    match key {
                        "escape" => {
                            this.open_picker = None;
                            cx.notify();
                        }
                        "down" | "j" if len > 0 => {
                            this.picker_highlight = (this.picker_highlight + 1) % len;
                            cx.notify();
                        }
                        "up" | "k" if len > 0 => {
                            this.picker_highlight = (this.picker_highlight + len - 1) % len;
                            cx.notify();
                        }
                        "enter" => {
                            let idx = this.picker_highlight;
                            this.confirm_picker_choice(which, idx, cx);
                        }
                        _ => {}
                    }
                    return;
                }
                if key == "escape" && this.is_dirty() {
                    this.discard_changes(Some(w), cx);
                } else if (ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control)
                    && key.eq_ignore_ascii_case("s")
                {
                    this.commit_save(cx);
                }
            }))
            .relative()
            .size_full()
            .bg(p.panel)
            .overflow_hidden()
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .size_full()
                    .overflow_y_scroll()
                    .pt(px(46.))
                    .pb(px(scroll_pb))
                    .px(px(pad_x))
                    .child(probe)
                    .child(
                        div()
                            .w_full()
                            .max_w(px(760.))
                            .mx_auto()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .line_height(px(32.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Preferences", "偏好设置")),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.text)
                                    .child(tr(
                                        cx,
                                        "Make the assistant work the way you do.",
                                        "让助手按照你的工作习惯协作。",
                                    )),
                            )
                            // Models section
                            .child(
                                div()
                                    .mt(px(36.))
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Models", "模型默认项")),
                            )
                            .child(
                                div()

                                    .mt(px(12.))
                                    .rounded(px(12.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .flex_col()
                                    .child({
                                        let mut r = div()
                                            .min_h(px(60.))
                                            .py(px(10.))
                                            .pl(px(16.))
                                            .pr(px(12.))
                                            .flex();
                                        if narrow {
                                            r = r.flex_col().items_stretch().gap(px(10.)).px(px(14.));
                                        } else {
                                            r = r.items_center().justify_between().gap(px(24.));
                                        }
                                        r.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(1.))
                                                .min_w_0()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .line_height(px(20.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.strong)
                                                        .child(tr(cx, "Default model", "默认模型")),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(p.muted)
                                                        .child(tr(
                                                            cx,
                                                            "Picked whenever you start a new chat",
                                                            "每次发起新对话时默认选中",
                                                        )),
                                                ),
                                        )
                                        .child(chat_trigger)
                                    })
                                    .child(pro_view_transition(
                                        pro_eid(&self.id, "auto-banner-vtx"),
                                        if is_auto { "auto" } else { "manual" },
                                        auto_banner,
                                    ))
                                    .child({
                                        let mut r = div()
                                            .min_h(px(60.))
                                            .py(px(10.))
                                            .pl(px(16.))
                                            .pr(px(12.))
                                            .border_t_1()
                                            .border_color(p.line)
                                            .flex();
                                        if narrow {
                                            r = r.flex_col().items_stretch().gap(px(10.)).px(px(14.));
                                        } else {
                                            r = r.items_center().justify_between().gap(px(24.));
                                        }
                                        r.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(1.))
                                                .min_w_0()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .line_height(px(20.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.strong)
                                                        .child(tr(cx, "Image model", "图像模型")),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(p.muted)
                                                        .child(tr(
                                                            cx,
                                                            "Used when you ask for a picture",
                                                            "在你需要生成图片时使用",
                                                        )),
                                                ),
                                        )
                                        .child(img_trigger)
                                    }),
                            )
                            // Language and appearance card
                            .child(
                                div()

                                    .mt(px(12.))
                                    .rounded(px(12.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .flex_col()
                                    .child({
                                        let mut r = div()
                                            .min_h(px(60.))
                                            .py(px(10.))
                                            .pl(px(16.))
                                            .pr(px(12.))
                                            .flex();
                                        if narrow {
                                            r = r.flex_col().items_stretch().gap(px(10.)).px(px(14.));
                                        } else {
                                            r = r.items_center().justify_between().gap(px(24.));
                                        }
                                        r.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(1.))
                                                .min_w_0()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .line_height(px(20.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.strong)
                                                        .child(tr(cx, "Language", "语言")),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(p.muted)
                                                        .child(tr(
                                                            cx,
                                                            "For menus and the assistant's answers",
                                                            "用于界面菜单与助手的默认回复语言",
                                                        )),
                                                ),
                                        )
                                        .child(lang_trigger)
                                    })
                                    .child({
                                        let mut r = div()
                                            .min_h(px(60.))
                                            .py(px(10.))
                                            .pl(px(16.))
                                            .pr(px(12.))
                                            .border_t_1()
                                            .border_color(p.line)
                                            .flex();
                                        if narrow {
                                            r = r.flex_col().items_start().gap(px(10.)).px(px(14.));
                                        } else {
                                            r = r.items_center().justify_between().gap(px(24.));
                                        }
                                        r.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(1.))
                                                .min_w_0()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .line_height(px(20.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.strong)
                                                        .child(tr(cx, "Appearance", "外观")),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(p.muted)
                                                        .child(tr(
                                                            cx,
                                                            "System follows your device",
                                                            "跟随系统会自动匹配你的设备主题",
                                                        )),
                                                ),
                                        )
                                        .child(appearance_seg)
                                    }),
                            )
                            // App actions card
                            .child(
                                div()

                                    .mt(px(12.))
                                    .rounded(px(12.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .min_h(px(60.))
                                    .py(px(10.))
                                    .pl(px(16.))
                                    .pr(px(12.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(24.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .gap(px(1.))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .flex_wrap()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .line_height(px(20.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.strong)
                                                            .child(tr(
                                                                cx,
                                                                "Find app actions on its own",
                                                                "自动发现可用应用操作",
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .px(px(7.))
                                                            .py(px(1.))
                                                            .rounded_full()
                                                            .bg(p.chip)
                                                            .text_size(px(11.))
                                                            .line_height(px(16.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.muted)
                                                            .child("Beta"),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.muted)
                                                    .child(tr(
                                                        cx,
                                                        "The chat can use connected apps without you tagging them first",
                                                        "无需先用 @ 提及，对话可自动调用已连接的应用工具",
                                                    )),
                                            ),
                                    )
                                    .child(app_actions_switch),
                            )
                            // Chat tools section
                            .child(
                                div()
                                    .mt(px(36.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .line_height(px(20.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(tr(cx, "Chat tools", "对话工具")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(20.))
                                            .text_color(p.muted)
                                            .child(tr(
                                                cx,
                                                "Extra tools for your own chats. Agents bring their own.",
                                                "适用于你个人对话的扩展工具。智能体会自带其专属工具。",
                                            )),
                                    ),
                            )
                            .child(tools_grid)
                            // Custom instructions section
                            .child(
                                div()
                                    .mt(px(36.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .line_height(px(20.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(tr(cx, "Custom instructions", "自定义指令")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(20.))
                                            .text_color(p.muted)
                                            .child(tr(
                                                cx,
                                                "Added to every message you send, so you don't have to repeat yourself.",
                                                "会自动附带在你发送的每条消息中，免去重复说明。",
                                            )),
                                    ),
                            )
                            .child(
                                div()

                                    .mt(px(12.))
                                    .rounded(px(12.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .min_h(px(60.))
                                            .py(px(10.))
                                            .pl(px(16.))
                                            .pr(px(12.))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(24.))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(1.))
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .line_height(px(20.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.strong)
                                                            .child(tr(
                                                                cx,
                                                                "Use my instructions",
                                                                "启用我的自定义指令",
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .line_height(px(16.))
                                                            .text_color(p.muted)
                                                            .child(tr(
                                                                cx,
                                                                "Turn them off for a while without losing them",
                                                                "可暂时关闭指令而不丢失已填写内容",
                                                            )),
                                                    ),
                                            )
                                            .child(instructions_switch),
                                    )
                                    .child(
                                        div()
                                            .border_t_1()
                                            .border_color(p.line)
                                            .pt(px(12.))
                                            .pb(px(14.))
                                            .pl(px(if narrow { 14. } else { 16. }))
                                            .pr(px(if narrow { 14. } else { 12. }))
                                            .opacity(instr_alpha)
                                            .flex()
                                            .flex_col()
                                            .gap(px(14.))
                                            .child(about_field)
                                            .child(style_field),
                                    ),
                            )
                            // Memory section
                            .child(memory_section),
                    ),
            )
            .children(picker_overlay);

        let discard_btn = pro_btn(
            pro_eid(&self.id, "discard-btn"),
            tr(cx, "Discard", "放弃修改"),
            None,
            ProBtnStyle::Secondary,
            true,
            cx,
            |this, w, cx| {
                this.discard_changes(Some(w), cx);
            },
        );

        let save_label: SharedString = tr(cx, "Save changes", "保存更改").into();
        let save_bg = p.primary;
        let save_fg = p.on_primary;
        let save_hover = p.primary.opacity(0.88);
        let focus = p.focus;
        let save_base = div()
            .id(pro_eid(&self.id, "save-btn"))
            .role(Role::Button)
            .aria_label(save_label.clone())
            .when(valid, |d| d.tab_index(0))
            .h(px(28.))
            .px(px(11.))
            .rounded(px(7.))
            .bg(save_bg)
            .text_color(save_fg)
            .text_size(px(12.5))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .justify_center()
            .when(!valid, |d| d.opacity(0.4).cursor_default())
            .when(valid, move |d| {
                d.cursor_pointer()
                    .hover(move |s| s.bg(save_hover))
                    .focus_visible(move |s| s.border_1().border_color(focus))
            })
            .child(save_label);
        let save_btn = if valid {
            activate(save_base, cx, |this, _, cx| {
                this.commit_save(cx);
            })
            .into_any_element()
        } else {
            save_base.into_any_element()
        };

        let dirty_id = pro_eid(&self.id, "dirty-bar");
        let dirty_aria: SharedString = tr(cx, "Unsaved changes", "未保存更改").into();
        let dirty_inner = div()
            .w_full()
            .max_w(px(760.))
            .px(px(14.))
            .py(px(10.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(div().text_size(px(13.)).text_color(p.ink).child(tr(
                cx,
                "Unsaved changes",
                "有未保存的更改",
            )))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(discard_btn)
                    .child(save_btn),
            )
            .into_any_element();

        root = root
            .child(pro_presence(
                pro_eid(&self.id, "dirty-bar-pres"),
                dirty,
                0.22,
                0.18,
                move |amount| {
                    div()
                        .id(dirty_id)
                        .role(Role::Region)
                        .aria_label(dirty_aria)
                        .absolute()
                        .left(px(pad_x))
                        .right(px(pad_x))
                        .bottom(px(16. - 10. * (1. - amount)))
                        .opacity(amount)
                        .flex()
                        .justify_center()
                        .child(dirty_inner)
                        .into_any_element()
                },
            ))
            .child(pro_toast_banner(self.toast.clone()));

        root
    }
}

impl Render for SettingsPreferencesScreen {
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
        AppearanceChoice, SettingsPreferencesEvent, SettingsPreferencesScreen, WorkspaceModelOption,
    };
    use gpui::{AppContext as _, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn model_filtering_and_paused_instructions_preserve_text() {
        struct EventSink;

        let app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<SettingsPreferencesEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let events_sink = events.clone();

        let (screen, _sink) = app.update(|cx| {
            let screen = cx.new(|cx| {
                SettingsPreferencesScreen::new("test-settings-preferences", cx).with_providers(
                    vec![
                        WorkspaceModelOption::chat("auto", "Auto", "auto"),
                        WorkspaceModelOption::chat("sonnet", "Claude Sonnet 5.5", "claude"),
                        WorkspaceModelOption::chat("opus", "Claude Opus 4.6", "claude")
                            .with_enabled(false),
                        WorkspaceModelOption::image("gpt-img", "GPT Image 2", "openai"),
                    ],
                )
            });
            let sink = cx.new(|sub_cx| {
                sub_cx
                    .subscribe(
                        &screen,
                        move |_: &mut EventSink, _, ev: &SettingsPreferencesEvent, _| {
                            events_sink.borrow_mut().push(ev.clone());
                        },
                    )
                    .detach();
                EventSink
            });
            (screen, sink)
        });

        app.update(|cx| {
            screen.update(cx, |s, s_cx| {
                assert_eq!(s.available_chat_models().len(), 2);
                // Disabled workspace model cannot be picked.
                assert!(!s.select_default_model("Claude Opus 4.6", s_cx));
                assert_eq!(s.preferences().default_model.as_ref(), "Claude Sonnet 5.5");

                // Switching to Auto succeeds and emits Save.
                s.switch_to_auto(s_cx);
                assert_eq!(s.preferences().default_model.as_ref(), "Auto");

                // Pausing instructions preserves the 121-character default text.
                assert_eq!(s.preferences().instructions.about.chars().count(), 121);
                s.toggle_instructions_enabled(None, s_cx);
                assert!(!s.preferences().instructions.enabled);
                assert_eq!(s.preferences().instructions.about.chars().count(), 121);

                s.select_appearance(AppearanceChoice::Dark, s_cx);
                assert_eq!(s.preferences().appearance, AppearanceChoice::Dark);
            });
        });

        let log = events.borrow();
        assert!(log.iter().any(|e| matches!(
            e,
            SettingsPreferencesEvent::AppearanceChanged(AppearanceChoice::Dark)
        )));
    }

    #[test]
    fn memory_forget_and_clear_lifecycle() {
        struct EventSink;

        let app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<SettingsPreferencesEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let events_sink = events.clone();

        let (screen, _sink) = app.update(|cx| {
            let screen =
                cx.new(|cx| SettingsPreferencesScreen::new("test-preferences-memories", cx));
            let sink = cx.new(|sub_cx| {
                sub_cx
                    .subscribe(
                        &screen,
                        move |_: &mut EventSink, _, ev: &SettingsPreferencesEvent, _| {
                            events_sink.borrow_mut().push(ev.clone());
                        },
                    )
                    .detach();
                EventSink
            });
            (screen, sink)
        });

        app.update(|cx| {
            screen.update(cx, |s, s_cx| {
                assert_eq!(s.preferences().memories.len(), 3);
                assert!(!s.forget_memory("missing", s_cx));
                assert!(s.forget_memory("mem-2", s_cx));
                assert_eq!(s.preferences().memories.len(), 2);

                s.clear_memories(s_cx);
                assert!(s.preferences().memories.is_empty());
            });
        });

        let log = events.borrow();
        assert!(log.iter().any(
            |e| matches!(e, SettingsPreferencesEvent::MemoryForgotten(id) if id.as_ref() == "mem-2")
        ));
        assert!(log.contains(&SettingsPreferencesEvent::MemoriesCleared));
    }
}
