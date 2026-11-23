//! Workspace AI model routing, regional provider controls, custom endpoints, and BYOK API keys.
//!
//! Renders inside [`AppShellPreset::Settings`] with default chat, fallback, and image model
//! pickers, an interactive `Auto` drum wheel explainer, regional provider cards (`EU`, `Global`,
//! and `Custom`) with per-model access menus and capsule switches, plus masked API key management.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProTone, pro_app_tile, pro_badge, pro_btn, pro_ease, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_switch, pro_toast_banner,
            pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, OnceLock},
};
use web_time::Instant;

const SVG_KEY: &str = r#"<path d="M13.4998 13.5C15.1242 14.1962 15.9365 14.5443 16.7554 14.4351C16.8478 14.4227 16.9396 14.4061 17.0305 14.3852C17.8357 14.2003 18.4742 13.5894 19.7513 12.3676L19.9012 12.2242C20.8695 11.2559 21.3537 10.7717 21.4713 9.78342C21.5889 8.79514 21.3365 8.38218 20.8317 7.55626C20.3833 6.82247 19.7571 5.98593 18.8855 5.11433C18.0139 4.24272 17.1773 3.61654 16.4435 3.16808C15.6176 2.66332 15.2047 2.41094 14.2164 2.52851C13.2281 2.64609 12.7439 3.13027 11.7756 4.09863L11.6322 4.24843C10.4105 5.52557 9.7996 6.16413 9.61466 6.96941C9.59382 7.0602 9.57721 7.1519 9.56489 7.24423C9.45564 8.06322 9.80375 8.87548 10.5 10.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M10.5002 10.499L2.50021 18.4997V21.4997H5.50021V19.4997H7.50021V17.4997H9.50021L13.5002 13.4997" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17.0002 7L16.0002 8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

fn eu_flag_badge() -> AnyElement {
    static EU_FLAG: OnceLock<Arc<Image>> = OnceLock::new();
    let img_arc = EU_FLAG
        .get_or_init(|| {
            let doc = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="28" viewBox="0 0 20 14"><rect width="20" height="14" rx="2.5" fill="#1c3f94"/><circle cx="10" cy="2.8" r="0.78" fill="#ffcc00"/><circle cx="12.1" cy="3.36" r="0.78" fill="#ffcc00"/><circle cx="13.64" cy="4.9" r="0.78" fill="#ffcc00"/><circle cx="14.2" cy="7.0" r="0.78" fill="#ffcc00"/><circle cx="13.64" cy="9.1" r="0.78" fill="#ffcc00"/><circle cx="12.1" cy="10.64" r="0.78" fill="#ffcc00"/><circle cx="10" cy="11.2" r="0.78" fill="#ffcc00"/><circle cx="7.9" cy="10.64" r="0.78" fill="#ffcc00"/><circle cx="6.36" cy="9.1" r="0.78" fill="#ffcc00"/><circle cx="5.8" cy="7.0" r="0.78" fill="#ffcc00"/><circle cx="6.36" cy="4.9" r="0.78" fill="#ffcc00"/><circle cx="7.9" cy="3.36" r="0.78" fill="#ffcc00"/></svg>"##;
            Arc::new(Image::from_bytes(ImageFormat::Svg, doc.as_bytes().to_vec()))
        })
        .clone();
    img(img_arc)
        .w(px(18.))
        .h(px(13.))
        .flex_none()
        .rounded(px(3.))
        .into_any_element()
}

fn provider_color_glyph(logo: &str, side: f32, cx: &App) -> AnyElement {
    static SPECIAL: [OnceLock<Arc<Image>>; 3] = [const { OnceLock::new() }; 3];
    let lower = logo.trim().to_ascii_lowercase();
    let matched = if lower.contains("anthropic") || lower.contains("claude") {
        Some((
            0,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"><path fill="#D97757" d="m4.7144 15.9555 4.7174-2.6471.079-.2307-.079-.1275h-.2307l-.7893-.0486-2.6956-.0729-2.3375-.0971-2.2646-.1214-.5707-.1215-.5343-.7042.0546-.3522.4797-.3218.686.0608 1.5179.1032 2.2767.1578 1.6514.0972 2.4468.255h.3886l.0546-.1579-.1336-.0971-.1032-.0972L6.973 9.8356l-2.55-1.6879-1.3356-.9714-.7225-.4918-.3643-.4614-.1578-1.0078.6557-.7225.8803.0607.2246.0607.8925.686 1.9064 1.4754 2.4893 1.8336.3643.3035.1457-.1032.0182-.0728-.164-.2733-1.3539-2.4467-1.445-2.4893-.6435-1.032-.17-.6194c-.0607-.255-.1032-.4674-.1032-.7285L6.287.1335 6.6997 0l.9957.1336.419.3642.6192 1.4147 1.0018 2.2282 1.5543 3.0296.4553.8985.2429.8318.091.255h.1579v-.1457l.1275-1.706.2368-2.0947.2307-2.6957.0789-.7589.3764-.9107.7468-.4918.5828.2793.4797.686-.0668.4433-.2853 1.8517-.5586 2.9021-.3643 1.9429h.2125l.2429-.2429.9835-1.3053 1.6514-2.0643.7286-.8196.85-.9046.5464-.4311h1.0321l.759 1.1293-.34 1.1657-1.0625 1.3478-.8804 1.1414-1.2628 1.7-.7893 1.36.0729.1093.1882-.0183 2.8535-.607 1.5421-.2794 1.8396-.3157.8318.3886.091.3946-.3278.8075-1.967.4857-2.3072.4614-3.4364.8136-.0425.0304.0486.0607 1.5482.1457.6618.0364h1.621l3.0175.2247.7892.522.4736.6376-.079.4857-1.2142.6193-1.6393-.3886-3.825-.9107-1.3113-.3279h-.1822v.1093l1.0929 1.0686 2.0035 1.8092 2.5075 2.3314.1275.5768-.3218.4554-.34-.0486-2.2039-1.6575-.85-.7468-1.9246-1.621h-.1275v.17l.4432.6496 2.3436 3.5214.1214 1.0807-.17.3521-.6071.2125-.6679-.1214-1.3721-1.9246L14.38 17.959l-1.1414-1.9428-.1397.079-.674 7.2552-.3156.3703-.7286.2793-.6071-.4614-.3218-.7468.3218-1.4753.3886-1.9246.3157-1.53.2853-1.9004.17-.6314-.0121-.0425-.1397.0182-1.4328 1.9672-2.1796 2.9446-1.7243 1.8456-.4128.164-.7164-.3704.0667-.6618.4008-.5889 2.386-3.0357 1.4389-1.882.929-1.0868-.0062-.1579h-.0546l-6.3385 4.1164-1.1293.1457-.4857-.4554.0608-.7467.2307-.2429 1.9064-1.3114Z"/></svg>"##,
        ))
    } else if lower.contains("google") || lower.contains("gemini") || lower.contains("imagen") {
        Some((
            1,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="24" x2="24" y2="0" gradientUnits="userSpaceOnUse"><stop stop-color="#1C7DFF"/><stop offset="0.52" stop-color="#1C69FF"/><stop offset="1" stop-color="#B04DFF"/></linearGradient></defs><path fill="url(#g)" d="M11.04 19.32Q12 21.51 12 24q0-2.49.93-4.68.96-2.19 2.58-3.81t3.81-2.55Q21.51 12 24 12q-2.49 0-4.68-.93a12.3 12.3 0 0 1-3.81-2.58 12.3 12.3 0 0 1-2.58-3.81Q12 2.49 12 0q0 2.49-.96 4.68-.93 2.19-2.55 3.81a12.3 12.3 0 0 1-3.81 2.58Q2.49 12 0 12q2.49 0 4.68.96 2.19.93 3.81 2.55t2.55 3.81"/></svg>"##,
        ))
    } else if lower.contains("mistral") {
        Some((
            2,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="-2 -2 29 29"><rect x="0" y="0" width="5" height="5" fill="#FFD800"/><rect x="20" y="0" width="5" height="5" fill="#FFD800"/><rect x="0" y="5" width="5" height="5" fill="#FFAF00"/><rect x="5" y="5" width="5" height="5" fill="#FFAF00"/><rect x="15" y="5" width="5" height="5" fill="#FFAF00"/><rect x="20" y="5" width="5" height="5" fill="#FFAF00"/><rect x="0" y="10" width="5" height="5" fill="#FF8205"/><rect x="5" y="10" width="5" height="5" fill="#FF8205"/><rect x="10" y="10" width="5" height="5" fill="#FF8205"/><rect x="15" y="10" width="5" height="5" fill="#FF8205"/><rect x="20" y="10" width="5" height="5" fill="#FF8205"/><rect x="0" y="15" width="5" height="5" fill="#FA500F"/><rect x="10" y="15" width="5" height="5" fill="#FA500F"/><rect x="20" y="15" width="5" height="5" fill="#FA500F"/><rect x="0" y="20" width="5" height="5" fill="#E10500"/><rect x="20" y="20" width="5" height="5" fill="#E10500"/></svg>"##,
        ))
    } else {
        None
    };

    if let Some((slot, markup)) = matched {
        let image = SPECIAL[slot]
            .get_or_init(|| {
                Arc::new(Image::from_bytes(
                    ImageFormat::Svg,
                    markup.as_bytes().to_vec(),
                ))
            })
            .clone();
        return img(image).size(px(side)).flex_none().into_any_element();
    }
    if let Some(mark) = pro_app_artwork(logo, side, cx) {
        return mark;
    }
    let p = ProPalette::get(cx);
    pro_icon(ProIcon::Terminal, side, p.muted)
}

fn provider_logo_tile(logo: &str, cx: &App) -> AnyElement {
    let p = ProPalette::get(cx);
    let bg: Hsla = if p.dark {
        rgb(0x2a2a2e).into()
    } else {
        rgb(0xffffff).into()
    };
    div()
        .size(px(28.))
        .flex_none()
        .rounded(px(8.))
        .bg(bg)
        .border_1()
        .border_color(p.line_strong)
        .flex()
        .items_center()
        .justify_center()
        .child(provider_color_glyph(logo, 15., cx))
        .into_any_element()
}

/// Hosting residency region for a model provider group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HostingRegion {
    /// Zero-retention European hosting (`Frankfurt`).
    #[default]
    Eu,
    /// Provider-managed global multi-region hosting.
    Global,
    /// Workspace-configured OpenAI-compatible endpoint.
    Custom,
}

impl HostingRegion {
    fn title(self, cx: &App) -> &'static str {
        match self {
            Self::Eu => tr(cx, "Hosted in the EU", "欧盟托管"),
            Self::Global => tr(cx, "Global", "全球节点"),
            Self::Custom => tr(cx, "Custom endpoints", "自定义端点"),
        }
    }

    fn note(self, cx: &App) -> &'static str {
        match self {
            Self::Eu => tr(
                cx,
                "Zero data retention, data stays in Frankfurt",
                "零数据留存，数据保留在法兰克福",
            ),
            Self::Global => tr(
                cx,
                "Runs where the provider runs it, mostly in the US",
                "在供应商原生计算区域运行（主要位于美国）",
            ),
            Self::Custom => tr(
                cx,
                "OpenAI-compatible endpoints managed by your workspace",
                "由工作区自行托管的 OpenAI 标准模型接口",
            ),
        }
    }
}

/// Modality category of a workspace model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelKind {
    /// Text and multimodal reasoning/chat model.
    #[default]
    Chat,
    /// Image generation model.
    Image,
}

/// Filter segment on the models toolbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelFilter {
    /// Show all chat, image, and custom models.
    #[default]
    All,
    /// Show chat models only.
    Chat,
    /// Show image models only.
    Image,
    /// Show custom self-hosted models only.
    Custom,
}

impl ModelFilter {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::Chat => tr(cx, "Chat", "对话"),
            Self::Image => tr(cx, "Image", "图像"),
            Self::Custom => tr(cx, "Custom", "自定义"),
        }
    }
}

/// Workspace role scope allowed to invoke a model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelAccess {
    /// Every workspace member and guest can select the model.
    #[default]
    Everyone,
    /// Restricted to workspace owners and admins.
    AdminsOnly,
    /// Restricted to members of the Engineering team.
    Engineering,
}

impl ModelAccess {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Everyone => tr(cx, "Everyone", "所有成员"),
            Self::AdminsOnly => tr(cx, "Admins only", "仅管理员"),
            Self::Engineering => tr(cx, "Engineering", "工程团队"),
        }
    }

    fn icon(self) -> ProIcon {
        match self {
            Self::Everyone => ProIcon::Users,
            Self::AdminsOnly => ProIcon::Shield,
            Self::Engineering => ProIcon::Code,
        }
    }
}

/// One configurable model entry inside a [`ModelProvider`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelEntry {
    pub id: SharedString,
    pub name: SharedString,
    pub subtitle: SharedString,
    pub kind: ModelKind,
    pub context_window: SharedString,
    pub pricing: SharedString,
    pub enabled: bool,
    pub access: ModelAccess,
}

impl ModelEntry {
    /// Create a model entry with context window and pricing metadata.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
        kind: ModelKind,
        context_window: impl Into<SharedString>,
        pricing: impl Into<SharedString>,
        enabled: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            subtitle: subtitle.into(),
            kind,
            context_window: context_window.into(),
            pricing: pricing.into(),
            enabled,
            access: ModelAccess::Everyone,
        }
    }

    /// Override who may select this model in the workspace.
    pub fn with_access(mut self, access: ModelAccess) -> Self {
        self.access = access;
        self
    }
}

/// Provider group containing one or more [`ModelEntry`] rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelProvider {
    pub id: SharedString,
    pub name: SharedString,
    pub logo: SharedString,
    pub region: HostingRegion,
    pub expanded: bool,
    pub models: Vec<ModelEntry>,
}

impl ModelProvider {
    /// Construct a provider card with its hosting region and initial model list.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        logo: impl Into<SharedString>,
        region: HostingRegion,
        expanded: bool,
        models: Vec<ModelEntry>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            logo: logo.into(),
            region,
            expanded,
            models,
        }
    }

    /// Count how many models in this provider are currently enabled.
    pub fn enabled_count(&self) -> usize {
        self.models.iter().filter(|m| m.enabled).count()
    }
}

/// Bring-your-own-key credential stored for direct provider billing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderApiKey {
    pub provider: SharedString,
    pub masked: SharedString,
    pub raw_preview: SharedString,
    pub added: SharedString,
    pub verified: bool,
}

impl ProviderApiKey {
    /// Construct a masked workspace API key record.
    pub fn new(
        provider: impl Into<SharedString>,
        masked: impl Into<SharedString>,
        added: impl Into<SharedString>,
    ) -> Self {
        let masked = masked.into();
        Self {
            provider: provider.into(),
            raw_preview: masked.clone(),
            masked,
            added: added.into(),
            verified: false,
        }
    }
}

/// Mask a raw API key string while preserving recognizable prefix and last 4 characters.
pub fn mask_api_key(raw: &str) -> String {
    let trimmed = raw.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        let tail: String = chars
            .iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return format!("••••••••{tail}");
    }
    let prefix_len = if trimmed.starts_with("sk-ant-") { 7 } else { 4 };
    let prefix: String = chars.iter().take(prefix_len).collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}••••••••{suffix}")
}

/// Snapshot of all workspace model settings emitted on save/change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSettingsSnapshot {
    pub default_chat: SharedString,
    pub default_fallback: SharedString,
    pub default_image: SharedString,
    pub providers: Vec<ModelProvider>,
    pub keys: Vec<ProviderApiKey>,
}

/// Events emitted by [`SettingsModelsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsModelsEvent {
    /// Sidebar or back navigation triggered.
    Navigate(SharedString),
    /// Any default model, switch, access scope, custom endpoint, or API key changed.
    Saved(ModelSettingsSnapshot),
    /// Connection test completed for a BYOK provider key.
    KeyTested { provider: SharedString, ok: bool },
}

impl EventEmitter<SettingsModelsEvent> for SettingsModelsScreen {}

/// Default sample providers matching the reference `Settings / Models` specification.
pub fn default_model_providers() -> Vec<ModelProvider> {
    vec![
        ModelProvider::new(
            "anthropic",
            "Anthropic",
            "Anthropic",
            HostingRegion::Eu,
            true,
            vec![
                ModelEntry::new(
                    "claude-opus-5-5",
                    "Claude Opus 5.5",
                    "Hosted by Anthropic in the EU",
                    ModelKind::Chat,
                    "200k",
                    "$15 / $75",
                    true,
                ),
                ModelEntry::new(
                    "claude-sonnet-5-5",
                    "Claude Sonnet 5.5",
                    "Hosted by Anthropic in the EU",
                    ModelKind::Chat,
                    "200k",
                    "$3 / $15",
                    true,
                ),
                ModelEntry::new(
                    "claude-haiku-5-5",
                    "Claude Haiku 5.5",
                    "Hosted by Anthropic in the EU",
                    ModelKind::Chat,
                    "200k",
                    "$0.80 / $4",
                    true,
                ),
            ],
        ),
        ModelProvider::new(
            "openai",
            "OpenAI",
            "OpenAI",
            HostingRegion::Eu,
            false,
            vec![
                ModelEntry::new(
                    "gpt-6",
                    "GPT-6",
                    "Hosted by OpenAI in the EU",
                    ModelKind::Chat,
                    "256k",
                    "$5 / $15",
                    true,
                ),
                ModelEntry::new(
                    "gpt-6-mini",
                    "GPT-6 mini",
                    "Hosted by OpenAI in the EU",
                    ModelKind::Chat,
                    "256k",
                    "$0.40 / $1.60",
                    true,
                ),
                ModelEntry::new(
                    "gpt-5-6",
                    "GPT-5.6",
                    "Hosted by OpenAI in the EU",
                    ModelKind::Chat,
                    "128k",
                    "$2.50 / $10",
                    false,
                )
                .with_access(ModelAccess::AdminsOnly),
                ModelEntry::new(
                    "gpt-image",
                    "GPT Image 2",
                    "Hosted by OpenAI in the EU",
                    ModelKind::Image,
                    "Image",
                    "$0.04 / img",
                    true,
                ),
            ],
        ),
        ModelProvider::new(
            "mistral",
            "Mistral",
            "Mistral",
            HostingRegion::Eu,
            false,
            vec![
                ModelEntry::new(
                    "mistral-large-3",
                    "Mistral Large 3",
                    "Hosted by Mistral in the EU",
                    ModelKind::Chat,
                    "128k",
                    "$2 / $6",
                    false,
                ),
                ModelEntry::new(
                    "mistral-medium-3",
                    "Mistral Medium 3",
                    "Hosted by Mistral in the EU",
                    ModelKind::Chat,
                    "128k",
                    "$0.40 / $2",
                    false,
                ),
            ],
        ),
        ModelProvider::new(
            "google",
            "Google",
            "Google Gemini",
            HostingRegion::Global,
            false,
            vec![
                ModelEntry::new(
                    "gemini-3-pro",
                    "Gemini 3 Pro",
                    "Hosted by Google",
                    ModelKind::Chat,
                    "2M",
                    "$1.25 / $5",
                    true,
                ),
                ModelEntry::new(
                    "gemini-3-flash",
                    "Gemini 3 Flash",
                    "Hosted by Google",
                    ModelKind::Chat,
                    "1M",
                    "$0.10 / $0.40",
                    false,
                ),
                ModelEntry::new(
                    "imagen-5",
                    "Imagen 5",
                    "Hosted by Google",
                    ModelKind::Image,
                    "Image",
                    "$0.03 / img",
                    false,
                ),
            ],
        ),
        ModelProvider::new(
            "xai",
            "xAI",
            "xAI",
            HostingRegion::Global,
            false,
            vec![
                ModelEntry::new(
                    "grok-4",
                    "Grok 4",
                    "Hosted by xAI",
                    ModelKind::Chat,
                    "256k",
                    "$3 / $15",
                    true,
                ),
                ModelEntry::new(
                    "grok-4-mini",
                    "Grok 4 mini",
                    "Hosted by xAI",
                    ModelKind::Chat,
                    "128k",
                    "$0.30 / $1.20",
                    false,
                )
                .with_access(ModelAccess::AdminsOnly),
            ],
        ),
    ]
}

/// Reconcile default model ids when a model is disabled (`chat` falls back to `"auto"`).
pub fn reconcile_model_defaults(
    providers: &[ModelProvider],
    default_chat: &mut SharedString,
    default_fallback: &mut SharedString,
    default_image: &mut SharedString,
) {
    let is_on = |target: &str, kind: ModelKind| {
        providers
            .iter()
            .flat_map(|p| p.models.iter())
            .any(|m| m.id.as_ref() == target && m.kind == kind && m.enabled)
    };

    if default_chat.as_ref() != "auto" && !is_on(default_chat.as_ref(), ModelKind::Chat) {
        *default_chat = "auto".into();
    }

    if !is_on(default_fallback.as_ref(), ModelKind::Chat) {
        *default_fallback = providers
            .iter()
            .flat_map(|p| p.models.iter())
            .find(|m| m.kind == ModelKind::Chat && m.enabled)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "auto".into());
    }

    if default_image.as_ref() != "auto" && !is_on(default_image.as_ref(), ModelKind::Image) {
        *default_image = providers
            .iter()
            .flat_map(|p| p.models.iter())
            .find(|m| m.kind == ModelKind::Image && m.enabled)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "auto".into());
    }
}

/// Filter providers and models against the active [`ModelFilter`] and search query.
pub fn filter_model_providers(
    providers: &[ModelProvider],
    filter: ModelFilter,
    query: &str,
) -> Vec<(usize, Vec<usize>)> {
    let q = query.trim().to_ascii_lowercase();
    let mut out = Vec::new();
    for (pi, prov) in providers.iter().enumerate() {
        if filter == ModelFilter::Custom && prov.region != HostingRegion::Custom {
            continue;
        }
        let prov_matches_q = !q.is_empty() && prov.name.to_ascii_lowercase().contains(&q);
        let mut model_indices = Vec::new();
        for (mi, m) in prov.models.iter().enumerate() {
            let kind_ok = match filter {
                ModelFilter::All | ModelFilter::Custom => true,
                ModelFilter::Chat => m.kind == ModelKind::Chat,
                ModelFilter::Image => m.kind == ModelKind::Image,
            };
            if !kind_ok {
                continue;
            }
            let text_ok = q.is_empty()
                || prov_matches_q
                || m.name.to_ascii_lowercase().contains(&q)
                || m.id.to_ascii_lowercase().contains(&q)
                || m.subtitle.to_ascii_lowercase().contains(&q);
            if text_ok {
                model_indices.push(mi);
            }
        }
        if !model_indices.is_empty() {
            out.push((pi, model_indices));
        }
    }
    out
}

#[derive(Clone, PartialEq, Eq)]
enum OpenMenu {
    ChatDefault,
    FallbackDefault,
    ImageDefault,
    Access(usize, usize),
    KeyProvider,
}

/// KUI Pro `Settings / Models` screen (`settings-models`).
pub struct SettingsModelsScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    loading: bool,
    empty: bool,
    providers: Vec<ModelProvider>,
    default_chat: SharedString,
    default_fallback: SharedString,
    default_image: SharedString,
    keys: Vec<ProviderApiKey>,
    revealed_keys: Vec<bool>,
    filter: ModelFilter,
    open_menu: Option<OpenMenu>,
    wheel_index: usize,
    wheel_prev: usize,
    wheel_anim: Option<Instant>,
    custom_form_open: bool,
    custom_kind: ModelKind,
    key_form_open: bool,
    key_provider: SharedString,
    key_test_banner: Option<SharedString>,
    toast: Option<(SharedString, Instant)>,
    search_editor: Option<Entity<Editor>>,
    custom_name_editor: Option<Entity<Editor>>,
    custom_url_editor: Option<Entity<Editor>>,
    key_secret_editor: Option<Entity<Editor>>,
    measured_width: Rc<Cell<f32>>,
    mounted_at: Instant,
}

impl SettingsModelsScreen {
    /// Construct a `Settings / Models` screen hosted inside [`AppShellPreset::Settings`].
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
                .with_active("Models")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsModelsEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsModelsEvent::Navigate("Back to app".into()));
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
            providers: default_model_providers(),
            default_chat: "auto".into(),
            default_fallback: "claude-sonnet-5-5".into(),
            default_image: "gpt-image".into(),
            keys: Vec::new(),
            revealed_keys: Vec::new(),
            filter: ModelFilter::All,
            open_menu: None,
            wheel_index: 0,
            wheel_prev: 0,
            wheel_anim: None,
            custom_form_open: false,
            custom_kind: ModelKind::Chat,
            key_form_open: false,
            key_provider: "Anthropic".into(),
            key_test_banner: None,
            toast: None,
            search_editor: None,
            custom_name_editor: None,
            custom_url_editor: None,
            key_secret_editor: None,
            measured_width: Rc::new(Cell::new(920.)),
            mounted_at: Instant::now(),
        }
    }

    /// Render only the inner settings page without the outer [`AppShell`] frame.
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

    /// Replace the provider list.
    pub fn with_providers(mut self, providers: Vec<ModelProvider>) -> Self {
        self.providers = providers;
        reconcile_model_defaults(
            &self.providers,
            &mut self.default_chat,
            &mut self.default_fallback,
            &mut self.default_image,
        );
        self
    }

    /// Set the initial default chat model id (or `"auto"`).
    pub fn with_default_chat(mut self, model_id: impl Into<SharedString>) -> Self {
        self.default_chat = model_id.into();
        self
    }

    /// Set the initial fallback chat model id.
    pub fn with_default_fallback(mut self, model_id: impl Into<SharedString>) -> Self {
        self.default_fallback = model_id.into();
        self
    }

    /// Set the initial default image generation model id.
    pub fn with_default_image(mut self, model_id: impl Into<SharedString>) -> Self {
        self.default_image = model_id.into();
        self
    }

    /// Populate workspace BYOK API keys.
    pub fn with_keys(mut self, keys: Vec<ProviderApiKey>) -> Self {
        self.revealed_keys = vec![false; keys.len()];
        self.keys = keys;
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Mutate skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty state (`No models yet` + `No API keys`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Mutate empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    fn emit_saved(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        reconcile_model_defaults(
            &self.providers,
            &mut self.default_chat,
            &mut self.default_fallback,
            &mut self.default_image,
        );
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(SettingsModelsEvent::Saved(ModelSettingsSnapshot {
            default_chat: self.default_chat.clone(),
            default_fallback: self.default_fallback.clone(),
            default_image: self.default_image.clone(),
            providers: self.providers.clone(),
            keys: self.keys.clone(),
        }));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search models…", "搜索模型…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                EditEvent::Escape => {
                    this.open_menu = None;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.custom_name_editor.is_none() {
            let ph: SharedString = tr(cx, "Model name (e.g. Llama 4 70B)", "模型名称").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Submitted => this.submit_custom_model_no_window(cx),
                EditEvent::Escape => {
                    this.custom_form_open = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.custom_name_editor = Some(ed);
        }

        if self.custom_url_editor.is_none() {
            let ph: SharedString = "https://inference.acme.internal/v1".into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Endpoint URL", InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Submitted => this.submit_custom_model_no_window(cx),
                EditEvent::Escape => {
                    this.custom_form_open = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.custom_url_editor = Some(ed);
        }

        if self.key_secret_editor.is_none() {
            let ph: SharedString = "sk-ant-api03-••••••••••••••••".into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("API key", InputMode::Password, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Submitted => this.submit_api_key_no_window(cx),
                EditEvent::Escape => {
                    this.key_form_open = false;
                    this.key_test_banner = None;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.key_secret_editor = Some(ed);
        }
    }

    fn submit_custom_model_no_window(&mut self, cx: &mut Context<Self>) {
        let name = self
            .custom_name_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            return;
        }
        let url = self
            .custom_url_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        let subtitle = if url.is_empty() {
            "https://inference.acme.internal/v1".to_string()
        } else {
            url
        };
        let slug: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let entry = ModelEntry::new(
            format!("custom-{slug}"),
            name.clone(),
            subtitle,
            self.custom_kind,
            if self.custom_kind == ModelKind::Chat {
                "128k"
            } else {
                "Image"
            },
            "Self-hosted",
            true,
        );
        if let Some(prov) = self
            .providers
            .iter_mut()
            .find(|p| p.region == HostingRegion::Custom)
        {
            prov.expanded = true;
            prov.models.push(entry);
        } else {
            self.providers.push(ModelProvider::new(
                "custom",
                "Self-hosted",
                "Custom",
                HostingRegion::Custom,
                true,
                vec![entry],
            ));
        }
        self.empty = false;
        self.custom_form_open = false;
        self.emit_saved(format!("Added {name}"), cx);
    }

    fn submit_custom_model(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.submit_custom_model_no_window(cx);
        if !self.custom_form_open {
            if let Some(ed) = &self.custom_name_editor {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
            if let Some(ed) = &self.custom_url_editor {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
        }
    }

    fn submit_api_key_no_window(&mut self, cx: &mut Context<Self>) {
        let raw = self
            .key_secret_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if raw.is_empty() {
            return;
        }
        let masked = mask_api_key(&raw);
        let prov = self.key_provider.clone();
        let key = ProviderApiKey {
            provider: prov.clone(),
            masked: masked.into(),
            raw_preview: raw.into(),
            added: "Added just now".into(),
            verified: self.key_test_banner.is_some(),
        };
        if let Some((idx, existing)) = self
            .keys
            .iter_mut()
            .enumerate()
            .find(|(_, k)| k.provider == prov)
        {
            *existing = key;
            if let Some(flag) = self.revealed_keys.get_mut(idx) {
                *flag = false;
            }
        } else {
            self.keys.push(key);
            self.revealed_keys.push(false);
        }
        self.key_form_open = false;
        self.key_test_banner = None;
        self.emit_saved(format!("Saved {prov} API key"), cx);
    }

    fn submit_api_key(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.submit_api_key_no_window(cx);
        if !self.key_form_open
            && let Some(ed) = &self.key_secret_editor
        {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "escape" => {
                if self.open_menu.take().is_some() || self.custom_form_open || self.key_form_open {
                    self.custom_form_open = false;
                    self.key_form_open = false;
                    self.key_test_banner = None;
                    cx.notify();
                }
            }
            "/" if !ev.keystroke.modifiers.platform && !ev.keystroke.modifiers.control => {
                if let Some(ed) = &self.search_editor {
                    let f = ed.read(cx).focus.clone();
                    if !f.is_focused(w) {
                        w.focus(&f, cx);
                        cx.stop_propagation();
                    }
                }
            }
            _ => {}
        }
    }

    fn find_model_display(&self, id: &str) -> Option<(SharedString, SharedString)> {
        for p in &self.providers {
            for m in &p.models {
                if m.id.as_ref() == id {
                    return Some((m.name.clone(), p.logo.clone()));
                }
            }
        }
        None
    }

    fn enabled_wheel_models(&self) -> Vec<(SharedString, SharedString)> {
        let mut items: Vec<(SharedString, SharedString)> = self
            .providers
            .iter()
            .flat_map(|p| {
                p.models
                    .iter()
                    .filter(|m| m.kind == ModelKind::Chat && m.enabled)
                    .map(|m| (m.name.clone(), p.logo.clone()))
            })
            .collect();
        if items.is_empty() {
            items.push(("Claude Opus 5.5".into(), "Anthropic".into()));
            items.push(("Claude Sonnet 5.5".into(), "Anthropic".into()));
            items.push(("Gemini 3 Pro".into(), "Google Gemini".into()));
        }
        items
    }

    fn render_model_wheel(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let items = self.enabled_wheel_models();
        let len = items.len();
        let center_idx = self.wheel_index % len;
        let slide_t = self
            .wheel_anim
            .map(|st| {
                if reduced {
                    1.0
                } else {
                    pro_ease((st.elapsed().as_secs_f32() / 0.34).clamp(0., 1.))
                }
            })
            .unwrap_or(1.0);
        let prev_delta = if self.wheel_prev == center_idx {
            0.0
        } else {
            (1.0 - slide_t) * 8.0
        };

        let mut drum = div()
            .w(px(if narrow { 220. } else { 232. }))
            .h(px(164.))
            .mx_auto()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.));

        for offset in -2i32..=2i32 {
            let idx = (center_idx as isize + offset as isize).rem_euclid(len as isize) as usize;
            let (name, logo) = &items[idx];
            let active = offset == 0;
            let alpha = match offset.abs() {
                0 => 1.0,
                1 => 0.46,
                _ => 0.18,
            };
            let fs = match offset.abs() {
                0 => 13.5,
                1 => 13.0,
                _ => 12.0,
            };

            let row = activate(
                div()
                    .id(pro_eid(&self.id, format!("wheel-slot-{}", offset + 2)))
                    .role(Role::Button)
                    .aria_label(name.clone())
                    .tab_index(0)
                    .h(px(if active { 34. } else { 28. }))
                    .px(px(12.))
                    .when(active, |d| {
                        d.mt(px(prev_delta * 0.25))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .opacity(alpha)
                    .cursor_pointer()
                    .child(provider_color_glyph(
                        logo.as_ref(),
                        if active { 15. } else { 13.5 },
                        cx,
                    ))
                    .child(
                        div()
                            .text_size(px(fs))
                            .font_weight(if active {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if active { p.strong } else { p.muted })
                            .whitespace_nowrap()
                            .child(name.clone()),
                    ),
                cx,
                move |this, _, cx| {
                    this.wheel_prev = this.wheel_index;
                    this.wheel_index = idx;
                    this.wheel_anim = Some(Instant::now());
                    cx.notify();
                },
            );
            drum = drum.child(row);
        }
        drum
    }

    fn render_defaults_card(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let chat_open = self.open_menu == Some(OpenMenu::ChatDefault);
        let fallback_open = self.open_menu == Some(OpenMenu::FallbackDefault);
        let image_open = self.open_menu == Some(OpenMenu::ImageDefault);

        let (chat_label, chat_logo): (SharedString, Option<SharedString>) =
            if self.default_chat.as_ref() == "auto" {
                (tr(cx, "Auto", "自动 (Auto)").into(), None)
            } else if let Some((n, logo)) = self.find_model_display(self.default_chat.as_ref()) {
                (n, Some(logo))
            } else {
                (tr(cx, "Auto", "自动 (Auto)").into(), None)
            };

        let (fallback_label, fallback_logo): (SharedString, Option<SharedString>) =
            if self.default_fallback.as_ref() == "auto" {
                (tr(cx, "Auto", "自动 (Auto)").into(), None)
            } else if let Some((n, logo)) = self.find_model_display(self.default_fallback.as_ref())
            {
                (n, Some(logo))
            } else {
                (tr(cx, "Auto", "自动 (Auto)").into(), None)
            };

        let (image_label, image_logo): (SharedString, Option<SharedString>) =
            if self.default_image.as_ref() == "auto" || self.empty {
                (tr(cx, "Auto", "自动 (Auto)").into(), None)
            } else if let Some((n, logo)) = self.find_model_display(self.default_image.as_ref()) {
                (n, Some(logo))
            } else {
                ("GPT Image 2".into(), Some("OpenAI".into()))
            };

        let chat_trigger = activate(
            div()
                .id(pro_eid(&self.id, "pick-chat"))
                .role(Role::Button)
                .aria_label(tr(cx, "Default chat model", "默认对话模型"))
                .tab_index(0)
                .w(px(if narrow { 240. } else { 200. }))
                .h(px(32.))
                .px(px(10.))
                .rounded(px(7.))
                .bg(p.card)
                .border_1()
                .border_color(if chat_open { p.ink } else { p.line_strong })
                .flex()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(if let Some(logo) = &chat_logo {
                    provider_color_glyph(logo.as_ref(), 15., cx)
                } else {
                    pro_icon(ProIcon::Sparkles, 15., p.muted)
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .child(chat_label),
                )
                .child(pro_icon(
                    if chat_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    13.,
                    p.muted,
                )),
            cx,
            |this, _, cx| {
                this.open_menu = if this.open_menu == Some(OpenMenu::ChatDefault) {
                    None
                } else {
                    Some(OpenMenu::ChatDefault)
                };
                cx.notify();
            },
        );

        let fallback_trigger = activate(
            div()
                .id(pro_eid(&self.id, "pick-fallback"))
                .role(Role::Button)
                .aria_label(tr(cx, "Fallback routing model", "故障回退模型"))
                .tab_index(0)
                .w(px(if narrow { 240. } else { 200. }))
                .h(px(32.))
                .px(px(10.))
                .rounded(px(7.))
                .bg(p.card)
                .border_1()
                .border_color(if fallback_open { p.ink } else { p.line_strong })
                .flex()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(if let Some(logo) = &fallback_logo {
                    provider_color_glyph(logo.as_ref(), 15., cx)
                } else {
                    pro_icon(ProIcon::GitBranch, 15., p.muted)
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .child(fallback_label),
                )
                .child(pro_icon(
                    if fallback_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    13.,
                    p.muted,
                )),
            cx,
            |this, _, cx| {
                this.open_menu = if this.open_menu == Some(OpenMenu::FallbackDefault) {
                    None
                } else {
                    Some(OpenMenu::FallbackDefault)
                };
                cx.notify();
            },
        );

        let image_trigger = activate(
            div()
                .id(pro_eid(&self.id, "pick-image"))
                .role(Role::Button)
                .aria_label(tr(cx, "Default image model", "默认图像模型"))
                .tab_index(0)
                .w(px(if narrow { 240. } else { 200. }))
                .h(px(32.))
                .px(px(10.))
                .rounded(px(7.))
                .bg(p.card)
                .border_1()
                .border_color(if image_open { p.ink } else { p.line_strong })
                .flex()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(if let Some(logo) = &image_logo {
                    provider_color_glyph(logo.as_ref(), 15., cx)
                } else {
                    pro_icon(ProIcon::Sparkles, 15., p.muted)
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .child(image_label),
                )
                .child(pro_icon(
                    if image_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    13.,
                    p.muted,
                )),
            cx,
            |this, _, cx| {
                this.open_menu = if this.open_menu == Some(OpenMenu::ImageDefault) {
                    None
                } else {
                    Some(OpenMenu::ImageDefault)
                };
                cx.notify();
            },
        );

        let frame_tint: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.024)
        };

        let mut card = div()
            .mt(px(12.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col();

        // Row 1: Chat model
        card = card.child(
            div()
                .min_h(px(60.))
                .pl(px(16.))
                .pr(px(12.))
                .py(px(10.))
                .flex()
                .when(narrow, |d| d.flex_col().items_start().gap(px(10.)))
                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
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
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Chat model", "对话模型")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Every new chat starts with it. People can pick their own in their preferences.",
                                    "每次新建对话默认启用，成员可在个人偏好中自行切换。",
                                )),
                        ),
                )
                .child(chat_trigger),
        );

        let chat_menu_el = self
            .render_default_menu(ModelKind::Chat, false, cx)
            .into_any_element();
        card = card.child(pro_fold(
            pro_eid(&self.id, "chat-def-menu-fold"),
            chat_open,
            160.,
            0.,
            move || chat_menu_el,
        ));

        // Auto explainer & 3D wheel
        let auto_explainer = div()
            .px(px(16.))
            .pb(px(12.))
            .child(
                div()
                    .px(px(20.))
                    .py(px(18.))
                    .rounded(px(10.))
                    .bg(frame_tint)
                    .flex()
                    .when(narrow, |d| d.flex_col().gap(px(16.)))
                    .when(!narrow, |d| d.items_center().justify_between().gap(px(28.)))
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
                                    .child(tr(cx, "How Auto works", "Auto 自动路由原理")),
                            )
                            .child(
                                div()
                                    .mt(px(3.))
                                    .mb(px(10.))
                                    .max_w(px(400.))
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Auto picks the model and how long it thinks for each message – the strongest one for hard questions, a fast one for quick replies.",
                                        "Auto 会根据每条消息动态匹配推理深度——复杂问题调用最强主力模型，日常问答即时响应。",
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
                                            .text_color(p.text)
                                            .child(pro_icon(ProIcon::Check, 13.5, p.muted))
                                            .child(tr(
                                                cx,
                                                "The best answer for what it costs",
                                                "平衡回答质量与推理成本",
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .text_size(px(12.5))
                                            .line_height(px(18.))
                                            .text_color(p.text)
                                            .child(pro_icon(ProIcon::Check, 13.5, p.muted))
                                            .child(tr(
                                                cx,
                                                "Only models you've turned on, hosted in the EU first",
                                                "仅调用已启用模型，优先欧盟零留存节点",
                                            )),
                                    ),
                            ),
                    )
                    .child(self.render_model_wheel(narrow, cx)),
            )
            .into_any_element();
        card = card.child(pro_fold(
            pro_eid(&self.id, "auto-wheel-fold"),
            self.default_chat.as_ref() == "auto",
            204.,
            0.,
            move || auto_explainer,
        ));

        // Row 2: Image model
        card = card.child(
            div()
                .min_h(px(60.))
                .pl(px(16.))
                .pr(px(12.))
                .py(px(10.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .when(narrow, |d| d.flex_col().items_start().gap(px(10.)))
                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
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
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Image model", "图像生成模型")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Used when someone asks for a picture",
                                    "成员请求生成图片时默认使用的模型",
                                )),
                        ),
                )
                .child(image_trigger),
        );

        let img_menu_el = self
            .render_default_menu(ModelKind::Image, false, cx)
            .into_any_element();
        card = card.child(pro_fold(
            pro_eid(&self.id, "img-def-menu-fold"),
            image_open,
            96.,
            0.,
            move || img_menu_el,
        ));

        // Row 3: Fallback routing model
        card = card.child(
            div()
                .min_h(px(60.))
                .pl(px(16.))
                .pr(px(12.))
                .py(px(10.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .when(narrow, |d| d.flex_col().items_start().gap(px(10.)))
                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
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
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Fallback routing", "故障回退路由")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Automatically routes retries when the primary provider is rate-limited",
                                    "当主供应商限流或不可达时自动切换备用模型",
                                )),
                        ),
                )
                .child(fallback_trigger),
        );

        let fb_menu_el = self
            .render_default_menu(ModelKind::Chat, true, cx)
            .into_any_element();
        card = card.child(pro_fold(
            pro_eid(&self.id, "fb-def-menu-fold"),
            fallback_open,
            140.,
            0.,
            move || fb_menu_el,
        ));

        card
    }

    fn render_default_menu(&self, kind: ModelKind, is_fallback: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let current = if is_fallback {
            self.default_fallback.as_ref()
        } else if kind == ModelKind::Chat {
            self.default_chat.as_ref()
        } else {
            self.default_image.as_ref()
        };

        let mut list = div()
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));

        if kind == ModelKind::Chat && !is_fallback {
            let selected = current == "auto";
            list = list.child(activate(
                div()
                    .id(pro_eid(&self.id, "opt-chat-auto"))
                    .role(Role::ListBoxOption)
                    .tab_index(0)
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .when(selected, |d| d.bg(p.select))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Sparkles, 14., p.ink))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.))
                            .font_weight(if selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(p.ink)
                            .child(tr(cx, "Auto (recommended)", "自动 (推荐)")),
                    )
                    .when(selected, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                cx,
                |this, _, cx| {
                    this.default_chat = "auto".into();
                    this.open_menu = None;
                    this.emit_saved("Default chat model set to Auto", cx);
                },
            ));
        }

        for prov in &self.providers {
            for m in &prov.models {
                if m.kind != kind || !m.enabled {
                    continue;
                }
                let mid = m.id.clone();
                let mname = m.name.clone();
                let selected = current == mid.as_ref();
                let suffix = if is_fallback {
                    format!("opt-fb-{mid}")
                } else {
                    format!("opt-def-{mid}")
                };
                list = list.child(activate(
                    div()
                        .id(pro_eid(&self.id, suffix))
                        .role(Role::ListBoxOption)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .when(selected, |d| d.bg(p.select))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .cursor_pointer()
                        .child(provider_color_glyph(prov.logo.as_ref(), 14., cx))
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
                                .child(mname.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.muted)
                                .child(m.context_window.clone()),
                        )
                        .when(selected, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                    cx,
                    move |this, _, cx| {
                        if is_fallback {
                            this.default_fallback = mid.clone();
                        } else if kind == ModelKind::Chat {
                            this.default_chat = mid.clone();
                        } else {
                            this.default_image = mid.clone();
                        }
                        this.open_menu = None;
                        this.emit_saved(format!("Default updated to {mname}"), cx);
                    },
                ));
            }
        }
        div().px(px(16.)).pb(px(10.)).child(list)
    }

    fn render_toolbar(&self, narrow: bool, query: &str, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let filters = [
            ModelFilter::All,
            ModelFilter::Chat,
            ModelFilter::Image,
            ModelFilter::Custom,
        ];
        let filter_key = match self.filter {
            ModelFilter::All => "all",
            ModelFilter::Chat => "chat",
            ModelFilter::Image => "image",
            ModelFilter::Custom => "custom",
        };
        let filter_sensor = pro_selection_sensor();
        let mut seg = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "model-filter-pill"),
                filter_key,
                filter_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));
        for (fi, f) in filters.iter().copied().enumerate() {
            let active = self.filter == f;
            seg = seg.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("filter-{fi}")))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(24.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .when(active, |d| d.child(pro_mark_selected(&filter_sensor)))
                    .hover(move |s| if active { s } else { s.bg(p.hover) })
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.ink } else { p.text })
                    .cursor_pointer()
                    .child(f.label(cx)),
                cx,
                move |this, _, cx| {
                    this.filter = f;
                    cx.notify();
                },
            ));
        }

        let search_box = div()
            .w(px(if narrow { 180. } else { 220. }))
            .when(narrow, |d| d.flex_1())
            .h(px(28.))
            .px(px(9.))
            .rounded(px(7.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_icon(ProIcon::Search, 14., p.muted))
            .when_some(self.search_editor.clone(), |d, ed| {
                d.child(div().flex_1().min_w_0().child(ed))
            })
            .when(!query.is_empty(), |d| {
                d.child(pro_icon_btn(
                    pro_eid(&self.id, "clear-search"),
                    tr(cx, "Clear search", "清空搜索"),
                    ProIcon::Close,
                    20.,
                    false,
                    cx,
                    |this, w, cx| {
                        if let Some(ed) = &this.search_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        cx.notify();
                    },
                ))
            });

        div()
            .mt(px(36.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(div().mr_auto().child(seg))
            .child(search_box)
            .child(pro_btn(
                pro_eid(&self.id, "btn-custom-model"),
                tr(cx, "Custom model", "自定义模型"),
                Some(ProIcon::Plus),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.custom_form_open = !this.custom_form_open;
                    cx.notify();
                },
            ))
    }

    fn render_custom_model_form(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let kinds = [
            (ModelKind::Chat, tr(cx, "Chat", "对话")),
            (ModelKind::Image, tr(cx, "Image", "图像")),
        ];
        let kind_sensor = pro_selection_sensor();
        let mut kind_seg = div()
            .relative()
            .p(px(2.))
            .rounded(px(7.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "custom-kind-pill"),
                if self.custom_kind == ModelKind::Chat {
                    "chat"
                } else {
                    "image"
                },
                kind_sensor.clone(),
                5.,
                p.card,
                Some(p.line),
                true,
            ));
        for (ki, (k, lbl)) in kinds.into_iter().enumerate() {
            let active = self.custom_kind == k;
            kind_seg = kind_seg.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("custom-kind-{ki}")))
                    .role(Role::RadioButton)
                    .tab_index(0)
                    .relative()
                    .h(px(24.))
                    .px(px(8.))
                    .rounded(px(5.))
                    .when(active, |d| d.child(pro_mark_selected(&kind_sensor)))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child(lbl),
                cx,
                move |this, _, cx| {
                    this.custom_kind = k;
                    cx.notify();
                },
            ));
        }

        div().pt(px(12.)).child(
            div()
                .p(px(16.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Add custom model", "添加自定义模型")),
                )
                .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                    cx,
                    "Connect any OpenAI-compatible chat or image endpoint.",
                    "接入任意标准 OpenAI 协议的对话或图像推理端点。",
                )))
                .child(
                    div()
                        .mt(px(8.))
                        .flex()
                        .when(narrow, |d| d.flex_col().items_start())
                        .when(!narrow, |d| d.items_center())
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .w_full()
                                .h(px(32.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .when_some(self.custom_name_editor.clone(), |d, ed| {
                                    d.child(div().w_full().child(ed))
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .w_full()
                                .h(px(32.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .when_some(self.custom_url_editor.clone(), |d, ed| {
                                    d.child(div().w_full().child(ed))
                                }),
                        )
                        .child(kind_seg)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_btn(
                                    pro_eid(&self.id, "custom-cancel"),
                                    tr(cx, "Cancel", "取消"),
                                    None,
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    |this, _, cx| {
                                        this.custom_form_open = false;
                                        cx.notify();
                                    },
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, "custom-submit"),
                                    tr(cx, "Add", "添加"),
                                    None,
                                    ProBtnStyle::Primary,
                                    true,
                                    cx,
                                    |this, w, cx| this.submit_custom_model(w, cx),
                                )),
                        ),
                ),
        )
    }

    fn render_provider_card(
        &self,
        pi: usize,
        model_indices: &[usize],
        searching: bool,
        narrow: bool,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let prov = &self.providers[pi];
        let is_open = prov.expanded || searching;
        let on_count = prov.enabled_count();
        let total_count = prov.models.len();
        let on_ratio = if total_count == 0 {
            0.0
        } else {
            (on_count as f32 / total_count as f32).clamp(0., 1.)
        };
        let prov_meter = pro_spring_bar(
            pro_eid(&self.id, format!("prov-meter-{}", prov.id)),
            on_ratio,
            4.,
            2.,
            p.chip,
            if on_count > 0 { p.blue } else { p.faint },
        );

        let head = activate(
            div()
                .id(pro_eid(&self.id, format!("prov-head-{}", prov.id)))
                .role(Role::Button)
                .aria_label(prov.name.clone())
                .tab_index(0)
                .min_h(px(56.))
                .px(px(14.))
                .flex()
                .items_center()
                .gap(px(10.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(provider_logo_tile(prov.logo.as_ref(), cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(prov.name.clone()),
                )
                .when(!narrow, |d| {
                    d.child(div().w(px(44.)).flex_none().child(prov_meter))
                })
                .child(
                    div()
                        .px(px(8.))
                        .h(px(20.))
                        .rounded_full()
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(format!("{on_count} / {total_count} on")),
                )
                .child(pro_icon(
                    if is_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    14.,
                    p.faint,
                )),
            cx,
            move |this, _, cx| {
                if let Some(target) = this.providers.get_mut(pi) {
                    target.expanded = !target.expanded;
                    cx.notify();
                }
            },
        );

        let mut rows_col = div().flex().flex_col();
        for &mi in model_indices {
            let model = &prov.models[mi];
            let access_open = self.open_menu == Some(OpenMenu::Access(pi, mi));
            let m_name = model.name.clone();
            let m_on = model.enabled;
            let access = model.access;

            let access_btn = activate(
                div()
                    .id(pro_eid(&self.id, format!("access-{pi}-{mi}")))
                    .role(Role::Button)
                    .aria_label(format!("Access for {m_name}"))
                    .tab_index(0)
                    .w(px(136.))
                    .h(px(28.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(if access_open { p.ink } else { p.line_strong })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(pro_icon(access.icon(), 13.5, p.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .child(access.label(cx)),
                    )
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                cx,
                move |this, _, cx| {
                    this.open_menu = if this.open_menu == Some(OpenMenu::Access(pi, mi)) {
                        None
                    } else {
                        Some(OpenMenu::Access(pi, mi))
                    };
                    cx.notify();
                },
            );

            let toggle_label = m_name.clone();
            let sw = pro_switch(
                pro_eid(&self.id, format!("sw-{pi}-{mi}")),
                format!("Enable {m_name}"),
                m_on,
                cx,
                move |this, _, cx| {
                    if let Some(entry) = this
                        .providers
                        .get_mut(pi)
                        .and_then(|pr| pr.models.get_mut(mi))
                    {
                        entry.enabled = !entry.enabled;
                        let state_word = if entry.enabled { "enabled" } else { "disabled" };
                        this.emit_saved(format!("{toggle_label} {state_word}"), cx);
                    }
                },
            );

            rows_col = rows_col.child(
                div()
                    .min_h(px(56.))
                    .pl(px(16.))
                    .pr(px(14.))
                    .py(px(9.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .when(narrow, |d| d.flex_col().items_start().gap(px(8.)))
                    .when(!narrow, |d| d.items_center().justify_between().gap(px(16.)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .when(!m_on, |d| d.opacity(0.5))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(model.name.clone()),
                                    )
                                    .when(model.kind == ModelKind::Image, |d| {
                                        d.child(pro_badge("Image", ProTone::Blue, false, cx))
                                    })
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .h(px(18.))
                                            .rounded(px(4.))
                                            .bg(p.chip)
                                            .flex()
                                            .items_center()
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .child(model.context_window.clone()),
                                    )
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .h(px(18.))
                                            .rounded(px(4.))
                                            .bg(p.chip)
                                            .flex()
                                            .items_center()
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .child(model.pricing.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(model.subtitle.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(access_btn)
                            .child(sw),
                    ),
            );

            let scopes = [
                ModelAccess::Everyone,
                ModelAccess::AdminsOnly,
                ModelAccess::Engineering,
            ];
            let acc_key = match access {
                ModelAccess::Everyone => "everyone",
                ModelAccess::AdminsOnly => "admins",
                ModelAccess::Engineering => "eng",
            };
            let acc_sensor = pro_selection_sensor();
            let mut menu = div()
                .relative()
                .p(px(4.))
                .rounded(px(8.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .gap(px(4.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, format!("acc-pill-{pi}-{mi}")),
                    acc_key,
                    acc_sensor.clone(),
                    6.,
                    p.card,
                    Some(p.line),
                    true,
                ));
            for (si, sc) in scopes.into_iter().enumerate() {
                let chosen = access == sc;
                let model_label = model.name.clone();
                menu = menu.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("acc-opt-{pi}-{mi}-{si}")))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .relative()
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .when(chosen, |d| d.child(pro_mark_selected(&acc_sensor)))
                        .hover(move |s| if chosen { s } else { s.bg(p.hover) })
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .child(pro_icon(sc.icon(), 13., p.muted))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(if chosen {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(p.ink)
                                .child(sc.label(cx)),
                        ),
                    cx,
                    move |this, _, cx| {
                        if let Some(entry) = this
                            .providers
                            .get_mut(pi)
                            .and_then(|pr| pr.models.get_mut(mi))
                        {
                            entry.access = sc;
                            this.open_menu = None;
                            this.emit_saved(format!("Updated access for {model_label}"), cx);
                        }
                    },
                ));
            }
            let acc_wrap = div().px(px(16.)).pb(px(8.)).child(menu).into_any_element();
            rows_col = rows_col.child(pro_fold(
                pro_eid(&self.id, format!("acc-fold-{pi}-{mi}")),
                access_open,
                46.,
                0.,
                move || acc_wrap,
            ));
        }

        let rows_el = rows_col.into_any_element();
        div()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(head)
            .child(pro_fold(
                pro_eid(&self.id, format!("prov-fold-{}", prov.id)),
                is_open,
                (model_indices.len().max(1) as f32) * 58.,
                0.,
                move || rows_el,
            ))
    }

    fn render_keys_section(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let key_prov_open = self.open_menu == Some(OpenMenu::KeyProvider);
        let secret_revealed = self
            .key_secret_editor
            .as_ref()
            .map(|e| e.read(cx).revealed)
            .unwrap_or(false);

        let mut section = div()
            .mt(px(36.))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .when(narrow, |d| d.flex_col().items_start().gap(px(10.)))
                    .when(!narrow, |d| d.items_end().justify_between().gap(px(24.)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "API keys", "API 密钥 (BYOK)")),
                            )
                            .child(
                                div()
                                    .max_w(px(580.))
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Bring your own key and that provider bills you directly – its usage no longer counts toward your plan.",
                                        "配置自有密钥后由供应商直接计费，相关调用不再消耗团队套餐额度。",
                                    )),
                            ),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "btn-add-key"),
                        tr(cx, "Add key", "添加密钥"),
                        Some(ProIcon::Plus),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.key_form_open = !this.key_form_open;
                            this.key_test_banner = None;
                            cx.notify();
                        },
                    )),
            );

        let prov_choices = ["Anthropic", "OpenAI", "Google Gemini", "Mistral", "xAI"];
        let mut prov_row = div().flex().flex_wrap().items_center().gap(px(6.));
        for (idx, name) in prov_choices.into_iter().enumerate() {
            let active = self.key_provider.as_ref() == name;
            prov_row = prov_row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("key-prov-opt-{idx}")))
                    .role(Role::MenuItem)
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .bg(if active { p.select } else { p.chip })
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .child(provider_color_glyph(name, 13., cx))
                    .child(div().text_size(px(12.)).text_color(p.ink).child(name)),
                cx,
                move |this, _, cx| {
                    this.key_provider = name.into();
                    this.open_menu = None;
                    this.key_test_banner = None;
                    cx.notify();
                },
            ));
        }
        let prov_row_el = prov_row.into_any_element();
        let key_form_el = div()
            .pt(px(12.))
            .child(
                div()
                    .p(px(16.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Add provider API key", "添加供应商 API 密钥")),
                            )
                            .when_some(self.key_test_banner.clone(), |d, msg| {
                                d.child(pro_badge(msg, ProTone::Green, true, cx))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .when(narrow, |d| d.flex_col().items_start())
                            .when(!narrow, |d| d.items_center())
                            .gap(px(8.))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "key-prov-trigger"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .w(px(if narrow { 220. } else { 164. }))
                                    .h(px(32.))
                                    .px(px(10.))
                                    .rounded(px(7.))
                                    .bg(p.subtle)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .cursor_pointer()
                                    .child(provider_color_glyph(
                                        self.key_provider.as_ref(),
                                        14.,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .flex_1()
                                            .truncate()
                                            .text_size(px(13.))
                                            .text_color(p.ink)
                                            .child(self.key_provider.clone()),
                                    )
                                    .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                                cx,
                                |this, _, cx| {
                                    this.open_menu =
                                        if this.open_menu == Some(OpenMenu::KeyProvider) {
                                            None
                                        } else {
                                            Some(OpenMenu::KeyProvider)
                                        };
                                    cx.notify();
                                },
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .w_full()
                                    .h(px(32.))
                                    .pl(px(10.))
                                    .pr(px(4.))
                                    .rounded(px(7.))
                                    .bg(p.subtle)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .when_some(self.key_secret_editor.clone(), |d, ed| {
                                        d.child(div().flex_1().min_w_0().child(ed))
                                    })
                                    .child(pro_icon_btn(
                                        pro_eid(&self.id, "toggle-key-reveal"),
                                        if secret_revealed {
                                            tr(cx, "Hide key", "隐藏密钥")
                                        } else {
                                            tr(cx, "Reveal key", "显示密钥")
                                        },
                                        ProIcon::Eye,
                                        24.,
                                        secret_revealed,
                                        cx,
                                        |this, _, cx| {
                                            if let Some(ed) = &this.key_secret_editor {
                                                ed.update(cx, |e, cx| {
                                                    e.revealed = !e.revealed;
                                                    cx.notify();
                                                });
                                            }
                                            cx.notify();
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "key-test"),
                                        tr(cx, "Test", "测试连接"),
                                        None,
                                        ProBtnStyle::Secondary,
                                        true,
                                        cx,
                                        |this, _, cx| {
                                            let prov = this.key_provider.clone();
                                            this.key_test_banner = Some("Connected · 38ms".into());
                                            cx.emit(SettingsModelsEvent::KeyTested {
                                                provider: prov,
                                                ok: true,
                                            });
                                            cx.notify();
                                        },
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "key-cancel"),
                                        tr(cx, "Cancel", "取消"),
                                        None,
                                        ProBtnStyle::Ghost,
                                        true,
                                        cx,
                                        |this, _, cx| {
                                            this.key_form_open = false;
                                            this.key_test_banner = None;
                                            cx.notify();
                                        },
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "key-save"),
                                        tr(cx, "Save key", "保存密钥"),
                                        None,
                                        ProBtnStyle::Primary,
                                        true,
                                        cx,
                                        |this, w, cx| this.submit_api_key(w, cx),
                                    )),
                            ),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, "key-prov-fold"),
                        key_prov_open,
                        32.,
                        10.,
                        move || prov_row_el,
                    )),
            )
            .into_any_element();
        section = section.child(pro_fold(
            pro_eid(&self.id, "key-form-fold"),
            self.key_form_open,
            118.,
            0.,
            move || key_form_el,
        ));

        if self.keys.is_empty() {
            let frame_tint: Hsla = if p.dark {
                hsla(0., 0., 1., 0.03)
            } else {
                hsla(0., 0., 0., 0.02)
            };
            section = section.child(
                div()
                    .mt(px(12.))
                    .py(px(44.))
                    .px(px(24.))
                    .rounded(px(12.))
                    .bg(frame_tint)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .size(px(42.))
                            .rounded(px(11.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 24 24", SVG_KEY, 18., p.muted)),
                    )
                    .child(
                        div()
                            .mt(px(14.))
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "No API keys", "尚未添加 API 密钥")),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .max_w(px(360.))
                            .text_center()
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "Usage runs on your plan until you add a provider's key.",
                                "在添加自有供应商密钥前，所有调用将计入团队套餐额度。",
                            )),
                    )
                    .child(div().mt(px(16.)).child(pro_btn(
                        pro_eid(&self.id, "empty-add-key"),
                        tr(cx, "Add key", "添加密钥"),
                        Some(ProIcon::Plus),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.key_form_open = true;
                            cx.notify();
                        },
                    ))),
            );
        } else {
            let mut card = div()
                .mt(px(12.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col();

            for (ki, key) in self.keys.iter().enumerate() {
                let shown = self.revealed_keys.get(ki).copied().unwrap_or(false);
                let prov_name = key.provider.clone();
                card = card.child(
                    div()
                        .min_h(px(54.))
                        .pl(px(16.))
                        .pr(px(12.))
                        .py(px(8.))
                        .when(ki > 0, |d| d.border_t_1().border_color(p.line))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .min_w_0()
                                .child(pro_app_tile(key.provider.as_ref(), 28., cx))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.strong)
                                                        .child(key.provider.clone()),
                                                )
                                                .when(key.verified, |d| {
                                                    d.child(pro_badge(
                                                        tr(cx, "Verified", "已验证"),
                                                        ProTone::Green,
                                                        false,
                                                        cx,
                                                    ))
                                                }),
                                        )
                                        .child(div().text_size(px(12.)).text_color(p.muted).child(
                                            if shown {
                                                key.raw_preview.clone()
                                            } else {
                                                key.masked.clone()
                                            },
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .when(!narrow, |d| {
                                    d.child(
                                        div()
                                            .mr(px(4.))
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(key.added.clone()),
                                    )
                                })
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, format!("key-eye-{ki}")),
                                    tr(cx, "Reveal masked key", "切换密钥显示"),
                                    ProIcon::Eye,
                                    28.,
                                    shown,
                                    cx,
                                    move |this, _, cx| {
                                        if let Some(flag) = this.revealed_keys.get_mut(ki) {
                                            *flag = !*flag;
                                            cx.notify();
                                        }
                                    },
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, format!("key-verify-{ki}")),
                                    tr(cx, "Test", "测试"),
                                    None,
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        if let Some(k) = this.keys.get_mut(ki) {
                                            k.verified = true;
                                            let pn = k.provider.clone();
                                            cx.emit(SettingsModelsEvent::KeyTested {
                                                provider: pn.clone(),
                                                ok: true,
                                            });
                                            this.toast = Some((
                                                format!("{pn} connection verified").into(),
                                                Instant::now(),
                                            ));
                                            cx.notify();
                                        }
                                    },
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, format!("key-del-{ki}")),
                                    format!("Remove {prov_name} key"),
                                    ProIcon::Trash,
                                    28.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        if ki < this.keys.len() {
                                            let removed = this.keys.remove(ki);
                                            if ki < this.revealed_keys.len() {
                                                this.revealed_keys.remove(ki);
                                            }
                                            this.emit_saved(
                                                format!("Removed {} key", removed.provider),
                                                cx,
                                            );
                                        }
                                    },
                                )),
                        ),
                );
            }
            section = section.child(card);
        }
        section
    }

    fn render_loading_skeleton(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .mt(px(36.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(pro_skeleton_bar(92., 14., 4., phase, cx))
            .child(
                div()
                    .p(px(16.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(pro_skeleton_bar(220., 16., 4., phase, cx))
                            .child(pro_skeleton_bar(180., 30., 7., phase, cx)),
                    )
                    .child(pro_skeleton_bar(320., 96., 10., phase, cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(pro_skeleton_bar(180., 16., 4., phase, cx))
                            .child(pro_skeleton_bar(180., 30., 7., phase, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(pro_skeleton_bar(190., 26., 7., phase, cx))
                    .child(pro_skeleton_bar(210., 26., 7., phase, cx)),
            )
            .children((0..3).map(|i| {
                div()
                    .p(px(16.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_skeleton_bar(28., 28., 8., phase + i as f32 * 0.1, cx))
                            .child(pro_skeleton_bar(130., 14., 4., phase, cx)),
                    )
                    .child(pro_skeleton_bar(64., 18., 9., phase, cx))
            }))
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;

        if let Some(st) = self.wheel_anim {
            if !reduced && st.elapsed().as_secs_f32() < 0.36 {
                w.request_animation_frame();
            } else {
                self.wheel_anim = None;
            }
        }
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
        let narrow = width < 600.;
        let query = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let searching = !query.trim().is_empty();

        let mut content = div()
            .w_full()
            .max_w(px(760.))
            .mx_auto()
            .pt(px(if narrow { 32. } else { 46. }))
            .pb(px(56.))
            .px(px(if narrow { 20. } else { 32. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(24.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Models", "模型")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Choose the models people in your workspace can use.",
                        "管理工作区成员可用的模型、路由规则与自有密钥。",
                    )),
            );

        if self.loading {
            content = content.child(self.render_loading_skeleton(skel_phase, cx));
        } else {
            content = content
                .child(
                    div()
                        .mt(px(36.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Defaults", "默认设置")),
                )
                .child(self.render_defaults_card(narrow, cx))
                .child(self.render_toolbar(narrow, &query, cx));

            let custom_form_el = self.render_custom_model_form(narrow, cx).into_any_element();
            content = content.child(pro_fold(
                pro_eid(&self.id, "custom-model-fold"),
                self.custom_form_open,
                120.,
                0.,
                move || custom_form_el,
            ));

            let filter_tag = match self.filter {
                ModelFilter::All => "all",
                ModelFilter::Chat => "chat",
                ModelFilter::Image => "image",
                ModelFilter::Custom => "custom",
            };
            let (list_key, list_body): (SharedString, AnyElement) = if self.empty {
                (
                    "empty".into(),
                    div()
                        .mt(px(20.))
                        .rounded(px(12.))
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Sparkles,
                            tr(cx, "No models yet", "暂无可用模型"),
                            tr(
                                cx,
                                "Connect a custom endpoint or enable workspace providers to make models selectable in chat.",
                                "添加自定义推理端点或开启模型供应商后，成员即可在对话中选用。",
                            ),
                            Some((
                                pro_eid(&self.id, "empty-custom-model"),
                                tr(cx, "Custom model", "自定义模型").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.custom_form_open = true;
                                    cx.notify();
                                }),
                            )),
                            cx,
                        ))
                        .into_any_element(),
                )
            } else {
                let filtered = filter_model_providers(&self.providers, self.filter, &query);
                if filtered.is_empty() {
                    (
                        format!("{filter_tag}-no-match").into(),
                        div()
                            .mt(px(20.))
                            .rounded(px(12.))
                            .border_1()
                            .border_color(p.line)
                            .child(pro_empty_state(
                                ProIcon::Search,
                                tr(cx, "No matching models", "未找到匹配的模型"),
                                tr(
                                    cx,
                                    "Try clearing your search or switching the model filter.",
                                    "请可改清空搜索词或切换上方筛选标签。",
                                ),
                                Some((
                                    pro_eid(&self.id, "reset-model-filters"),
                                    tr(cx, "Reset filters", "重置筛选").into(),
                                    Box::new(|this: &mut Self, w, cx| {
                                        this.filter = ModelFilter::All;
                                        if let Some(ed) = &this.search_editor {
                                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                                        }
                                        cx.notify();
                                    }),
                                )),
                                cx,
                            ))
                            .into_any_element(),
                    )
                } else {
                    let mut list_col = div().flex().flex_col();
                    for region in [
                        HostingRegion::Eu,
                        HostingRegion::Global,
                        HostingRegion::Custom,
                    ] {
                        let region_groups: Vec<&(usize, Vec<usize>)> = filtered
                            .iter()
                            .filter(|(pi, _)| self.providers[*pi].region == region)
                            .collect();
                        if region_groups.is_empty() {
                            continue;
                        }
                        let region_icon: AnyElement = match region {
                            HostingRegion::Eu => eu_flag_badge(),
                            HostingRegion::Global => pro_icon(ProIcon::Globe, 15., p.muted),
                            HostingRegion::Custom => pro_icon(ProIcon::Terminal, 15., p.muted),
                        };
                        let mut reg_sec = div().mt(px(28.)).flex().flex_col().gap(px(8.)).child(
                            div()
                                .mb(px(2.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(region_icon)
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(region.title(cx)),
                                )
                                .when(!narrow, |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(region.note(cx)),
                                    )
                                }),
                        );
                        for (pi, indices) in region_groups {
                            reg_sec = reg_sec.child(
                                self.render_provider_card(*pi, indices, searching, narrow, cx),
                            );
                        }
                        list_col = list_col.child(reg_sec);
                    }
                    (filter_tag.into(), list_col.into_any_element())
                }
            };
            content = content.child(pro_view_transition(
                pro_eid(&self.id, "models-list-vtx"),
                list_key,
                list_body,
            ));

            content = content.child(self.render_keys_section(narrow, cx));
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

impl Render for SettingsModelsScreen {
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
        HostingRegion, ModelFilter, ModelKind, default_model_providers, filter_model_providers,
        mask_api_key, reconcile_model_defaults,
    };
    use gpui::SharedString;

    #[test]
    fn disabling_default_chat_model_falls_back_to_auto() {
        let mut providers = default_model_providers();
        let mut default_chat: SharedString = "claude-opus-5-5".into();
        let mut default_fallback: SharedString = "claude-opus-5-5".into();
        let mut default_image: SharedString = "gpt-image".into();

        // Turn off Claude Opus 5.5 and GPT Image 2, enable Imagen 5
        providers[0].models[0].enabled = false;
        providers[1].models[3].enabled = false;
        providers[3].models[2].enabled = true;

        reconcile_model_defaults(
            &providers,
            &mut default_chat,
            &mut default_fallback,
            &mut default_image,
        );

        assert_eq!(default_chat.as_ref(), "auto");
        assert_eq!(default_fallback.as_ref(), "claude-sonnet-5-5");
        assert_eq!(default_image.as_ref(), "imagen-5");
    }

    #[test]
    fn model_filtering_and_byok_masking_match_contracts() {
        let providers = default_model_providers();
        let eu_count = providers
            .iter()
            .filter(|p| p.region == HostingRegion::Eu)
            .count();
        assert_eq!(eu_count, 3);

        let image_only = filter_model_providers(&providers, ModelFilter::Image, "");
        let total_images: usize = image_only.iter().map(|(_, idxs)| idxs.len()).sum();
        assert_eq!(total_images, 2);
        for (pi, idxs) in &image_only {
            for &mi in idxs {
                assert_eq!(providers[*pi].models[mi].kind, ModelKind::Image);
            }
        }

        let opus_query = filter_model_providers(&providers, ModelFilter::All, "opus");
        assert_eq!(opus_query.len(), 1);
        assert_eq!(opus_query[0].1, vec![0]);

        assert_eq!(
            mask_api_key("sk-ant-api03-9988776655449f2a"),
            "sk-ant-••••••••9f2a"
        );
        assert_eq!(mask_api_key("short99"), "••••••••99");
    }
}
