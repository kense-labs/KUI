//! AI assistant start screen with greeting, tool composer, connected apps, and starter prompts.
//!
//! Renders [`AssistantHomeScreen`] inside [`AppShell`] (`AppShellPreset::Chat`, active `"New chat"`),
//! pairing a centered greeting, multi-line prompt composer with approval and model selectors,
//! folder/app tray, and seven quick-action starter chips that transition smoothly into an active
//! streaming conversation thread with workspace artifacts.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
            pro_presence, pro_selection_sensor, pro_skeleton_bar, pro_switch, pro_view_transition,
            pro_view_transition_fill, tr,
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

const HAND_APPROVAL_SVG: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 32.2558 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SHIELD_ALERT_SVG: &str = r#"<path d="M33.2482 22.1795V33.27" stroke="currentColor" stroke-width="4.6" stroke-linecap="round" stroke-linejoin="round"></path><path d="M33.5966 43.67H33.2501M33.9432 43.67C33.9432 44.0529 33.633 44.3631 33.2501 44.3631C32.8672 44.3631 32.5569 44.0529 32.5569 43.67C32.5569 43.2871 32.8672 42.9768 33.2501 42.9768C33.633 42.9768 33.9432 43.2871 33.9432 43.67Z" stroke="currentColor" stroke-width="4.6" stroke-linecap="round" stroke-linejoin="round"></path><path d="M58.202 31.0087V22.9595C58.202 18.4124 58.202 16.1389 57.0816 14.6555C55.9612 13.1722 53.4278 12.452 48.3612 11.0118C44.8995 10.0278 41.848 8.84228 39.41 7.76002C36.0859 6.28445 34.4237 5.54666 33.2484 5.54666C32.0728 5.54666 30.4109 6.28445 27.0868 7.76002C24.6487 8.84228 21.5973 10.0278 18.1357 11.0118C13.069 12.452 10.5356 13.1722 9.4152 14.6555C8.29475 16.1389 8.29475 18.4124 8.29475 22.9595V31.0087C8.29475 46.605 22.3319 55.9626 29.3498 59.6666C31.0331 60.5549 31.8748 60.9991 33.2484 60.9991C34.622 60.9991 35.4635 60.5549 37.1467 59.6666C44.1648 55.9626 58.202 46.605 58.202 31.0087Z" stroke="currentColor" stroke-width="4.6" stroke-linecap="round"></path>"#;

const CHIP_KNOWLEDGE_SVG: &str = r#"<path d="M3 12C3 7.75736 3 5.63604 4.31802 4.31802C5.63604 3 7.75736 3 12 3C16.2426 3 18.364 3 19.682 4.31802C21 5.63604 21 7.75736 21 12C21 16.2426 21 18.364 19.682 19.682C18.364 21 16.2426 21 12 21C7.75736 21 5.63604 21 4.31802 19.682C3 18.364 3 16.2426 3 12Z" stroke="currentColor" stroke-width="1.9"></path><path d="M3 12H21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M11 7.5L17 7.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7.125 7.5H7M7.25 7.5C7.25 7.63807 7.13807 7.75 7 7.75C6.86193 7.75 6.75 7.63807 6.75 7.5C6.75 7.36193 6.86193 7.25 7 7.25C7.13807 7.25 7.25 7.36193 7.25 7.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M11 16.5L17 16.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7.125 16.5H7M7.25 16.5C7.25 16.6381 7.13807 16.75 7 16.75C6.86193 16.75 6.75 16.6381 6.75 16.5C6.75 16.3619 6.86193 16.25 7 16.25C7.13807 16.25 7.25 16.3619 7.25 16.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const CHIP_DOCUMENT_SVG: &str = r#"<path d="M8 7L16 7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M8 11L12 11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M13 21.5V21C13 18.1716 13 16.7574 13.8787 15.8787C14.7574 15 16.1716 15 19 15H19.5M20 13.3431V10C20 6.22876 20 4.34315 18.8284 3.17157C17.6569 2 15.7712 2 12 2C8.22877 2 6.34315 2 5.17157 3.17157C4 4.34314 4 6.22876 4 10L4 14.5442C4 17.7892 4 19.4117 4.88607 20.5107C5.06508 20.7327 5.26731 20.9349 5.48933 21.1139C6.58831 22 8.21082 22 11.4558 22C12.1614 22 12.5141 22 12.8372 21.886C12.9044 21.8623 12.9702 21.835 13.0345 21.8043C13.3436 21.6564 13.593 21.407 14.0919 20.9081L18.8284 16.1716C19.4065 15.5935 19.6955 15.3045 19.8478 14.9369C20 14.5694 20 14.1606 20 13.3431Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const CHIP_PRESENTATION_SVG: &str = r#"<path d="M12 18V22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M10 18L5 22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M14 18L19 22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M2.5 12C2.5 14.8284 2.5 16.2426 3.37868 17.1213C4.25736 18 5.67157 18 8.5 18H15.5C18.3284 18 19.7426 18 20.6213 17.1213C21.5 16.2426 21.5 14.8284 21.5 12V8C21.5 5.17157 21.5 3.75736 20.6213 2.87868C19.7426 2 18.3284 2 15.5 2H8.5C5.67157 2 4.25736 2 3.37868 2.87868C2.5 3.75736 2.5 5.17157 2.5 8V12Z" stroke="currentColor" stroke-width="1.9"></path><path d="M8 13V10M12 13V7M16 13V11" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path>"#;

const CHIP_TABLE_SVG: &str = r#"<path d="M20 10C20 10 22.375 11.8222 21.375 14.6667C20.375 17.5111 21.375 19.4074 22 20M20 10H4M20 10V4M4 10C4 10 1.625 11.8222 2.625 14.6667C3.625 17.5111 2.625 19.4074 2 20M4 10V4M22 4H20.75H20M2 4H3.25H4M4 4H20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M11 7H13" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const CHIP_IMAGE_SVG: &str = r#"<circle cx="7.5" cy="7.5" r="1.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle><path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" stroke="currentColor" stroke-width="1.9"></path><path d="M5 21C9.37246 15.775 14.2741 8.88406 21.4975 13.5424" stroke="currentColor" stroke-width="1.9"></path>"#;

const CHIP_RESEARCH_SVG: &str = r#"<path d="M16.1995 2.62118C17.0064 2.153 17.4099 1.91892 17.8053 2.02539C18.2007 2.13186 18.4336 2.53732 18.8995 3.34823L20.3819 5.92852C20.8478 6.73943 21.0807 7.14489 20.9747 7.54225C20.8688 7.93961 20.4653 8.1737 19.6584 8.64188L16.235 10.6282C15.4281 11.0964 15.0247 11.3305 14.6293 11.224C14.2339 11.1175 14.0009 10.7121 13.5351 9.90115L12.0527 7.32085C11.5868 6.50994 11.3539 6.10449 11.4598 5.70713C11.5658 5.30977 11.9692 5.07568 12.7761 4.6075L16.1995 2.62118Z" stroke="currentColor" stroke-width="1.9"></path><path d="M11.5585 6.46075L14.0292 10.7613L10.6058 12.7476C9.79886 13.2157 9.39541 13.4498 9.00001 13.3434C8.60461 13.2369 8.37167 12.8314 7.9058 12.0205L7.41168 11.1604C6.94581 10.3495 6.71288 9.94406 6.81882 9.5467C6.92477 9.14934 7.32822 8.91525 8.13513 8.44707L11.5585 6.46075Z" stroke="currentColor" stroke-width="1.9"></path><path d="M6.91755 10.3003L8.39993 12.8806L5.40444 14.6186C5.00566 14.85 4.80627 14.9657 4.61138 14.9915C4.35152 15.0259 4.08871 14.9551 3.88077 14.7948C3.72482 14.6745 3.6097 14.4741 3.37947 14.0734C3.14923 13.6726 3.03412 13.4722 3.00846 13.2764C2.97425 13.0152 3.04467 12.7511 3.20422 12.5421C3.32389 12.3854 3.52328 12.2697 3.92206 12.0384L6.91755 10.3003Z" stroke="currentColor" stroke-width="1.9"></path><path d="M7.5 22L12 12L16.5 22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const CHIP_VISUALIZE_SVG: &str = r#"<path d="M8 9V17" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M13 5V17" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M18 13V17" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3 3V13C3 16.7712 3 18.6569 4.17157 19.8284C5.34315 21 7.22876 21 11 21H21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const ART_DOC_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

const ART_SHEET_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;

fn tray_folder_icon(side: f32) -> AnyElement {
    static FOLDER_IMG: OnceLock<Arc<Image>> = OnceLock::new();
    const MARKUP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 67 67" width="48" height="48" fill="none"><path d="M11.17 15.35v15.36h44.67v-6.98c0-3.86-3.13-6.98-6.98-6.98H33.5l-3.93-5.89a5.58 5.58 0 0 0-4.64-2.49h-6.78c-3.86 0-6.98 3.13-6.98 6.98Z" fill="#00A1FF"></path><path d="M33.5 16.75l-3.93-5.89a5.58 5.58 0 0 0-4.64-2.49h-6.78c-3.86 0-6.98 3.13-6.98 6.98v15.36h44.67v-6.98c0-3.86-3.13-6.98-6.98-6.98H33.5Zm0 0h-9.77" stroke="#00A1FF" stroke-width="4.19" stroke-linecap="round" stroke-linejoin="round"></path><path d="M6.59 42.38c-1.05-5.26-1.57-7.89-.08-9.67l.31-.34c1.66-1.66 4.59-1.66 10.45-1.66h32.46c5.86 0 8.79 0 10.45 1.66l.31.34c1.49 1.78.97 4.41-.08 9.67-1.51 7.56-2.26 11.34-5.1 13.66l-.6.46c-3.01 2.13-7.23 2.13-15.66 2.13h-11.1c-8.43 0-12.64 0-15.66-2.13l-.6-.46c-2.84-2.32-3.6-6.1-5.1-13.66Z" fill="#3CC7FF" stroke="#3CC7FF" stroke-width="4.19"></path></svg>"##;
    let image = FOLDER_IMG
        .get_or_init(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                MARKUP.as_bytes().to_vec(),
            ))
        })
        .clone();
    img(image).size(px(side)).flex_none().into_any_element()
}

fn clickup_mark(side: f32) -> AnyElement {
    static CLICKUP_IMG: OnceLock<Arc<Image>> = OnceLock::new();
    const MARKUP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 49.7096 59.6884" width="48" height="48" fill="none"><path fill-rule="evenodd" clip-rule="evenodd" d="M0 45.9002L9.1618 38.8247C14.0602 45.1745 19.2307 48.1679 24.9455 48.1679C30.6603 48.1679 35.7401 45.2652 40.3663 38.9154L49.7096 45.8094C42.997 54.8805 34.6516 59.6882 24.9455 59.6882C15.3301 59.6882 6.89403 54.8805 0 45.9002Z" fill="url(#cu0)"></path><path fill-rule="evenodd" clip-rule="evenodd" d="M24.9535 15.3301L8.62558 29.3903L1.09657 20.5914L25.0442 0L48.8105 20.5914L41.1908 29.2996L24.9535 15.3301Z" fill="url(#cu1)"></path><defs><linearGradient id="cu0" x1="0" y1="49.2666" x2="49.75" y2="49.2666" gradientUnits="userSpaceOnUse"><stop stop-color="#8930FD"></stop><stop offset="1" stop-color="#49CCF9"></stop></linearGradient><linearGradient id="cu1" x1="1.09231" y1="14.6901" x2="48.7603" y2="14.6901" gradientUnits="userSpaceOnUse"><stop stop-color="#FF02F0"></stop><stop offset="1" stop-color="#FFC800"></stop></linearGradient></defs></svg>"##;
    let image = CLICKUP_IMG
        .get_or_init(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                MARKUP.as_bytes().to_vec(),
            ))
        })
        .clone();
    img(image).size(px(side)).flex_none().into_any_element()
}

/// Action approval policy selected in the prompt composer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssistantApprovalMode {
    #[default]
    RequestApproval,
    AutoApproveSafe,
    FullAccess,
}

impl AssistantApprovalMode {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "请求审批"),
            Self::AutoApproveSafe => tr(cx, "Auto-approve safe", "只读自动批准"),
            Self::FullAccess => tr(cx, "Full access", "完全自动执行"),
        }
    }
}

/// Model option offered in the composer model selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistantModelOption {
    pub id: SharedString,
    pub name: SharedString,
    pub provider: Option<SharedString>,
    pub summary: SharedString,
}

impl AssistantModelOption {
    /// Construct a selectable model entry (`provider = None` renders `"Auto"`).
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        provider: Option<&str>,
        summary: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider: provider.map(|s| s.to_owned().into()),
            summary: summary.into(),
        }
    }
}

/// File or app context chip staged inside the prompt composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistantAttachment {
    pub name: SharedString,
    pub detail: SharedString,
}

impl AssistantAttachment {
    /// Construct an attachment chip with a display name and secondary detail.
    pub fn new(name: impl Into<SharedString>, detail: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            detail: detail.into(),
        }
    }
}

/// Visual icon category for a quick starter prompt chip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StarterChipKind {
    #[default]
    CompanyKnowledge,
    CreateDocument,
    CreatePresentation,
    CreateTable,
    GenerateImage,
    DeepResearch,
    VisualizeData,
}

impl StarterChipKind {
    fn svg(self) -> &'static str {
        match self {
            Self::CompanyKnowledge => CHIP_KNOWLEDGE_SVG,
            Self::CreateDocument => CHIP_DOCUMENT_SVG,
            Self::CreatePresentation => CHIP_PRESENTATION_SVG,
            Self::CreateTable => CHIP_TABLE_SVG,
            Self::GenerateImage => CHIP_IMAGE_SVG,
            Self::DeepResearch => CHIP_RESEARCH_SVG,
            Self::VisualizeData => CHIP_VISUALIZE_SVG,
        }
    }
}

/// One quick-action suggestion chip displayed under the start composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StarterPromptChip {
    pub id: SharedString,
    pub label: SharedString,
    pub prompt: SharedString,
    pub kind: StarterChipKind,
}

impl StarterPromptChip {
    /// Construct a starter prompt chip.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        prompt: impl Into<SharedString>,
        kind: StarterChipKind,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            prompt: prompt.into(),
            kind,
        }
    }
}

/// Generated workspace artifact chip attached to an assistant reply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedArtifactChip {
    pub name: SharedString,
    pub is_sheet: bool,
}

/// One turn in the active conversation after submitting a prompt or starter chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssistantTurn {
    User {
        id: SharedString,
        text: SharedString,
        folder: Option<SharedString>,
        attachments: Vec<AssistantAttachment>,
    },
    Assistant {
        id: SharedString,
        trace_summary: SharedString,
        trace_steps: Vec<SharedString>,
        text: SharedString,
        artifact: Option<GeneratedArtifactChip>,
        streaming: bool,
    },
}

/// Events emitted by [`AssistantHomeScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssistantHomeEvent {
    Navigate(SharedString),
    SubmitPrompt {
        prompt: SharedString,
        model_id: SharedString,
        approval: AssistantApprovalMode,
        folder: Option<SharedString>,
        attachments: Vec<AssistantAttachment>,
    },
    SelectStarter(SharedString),
    SelectModel(SharedString),
    SelectFolder(Option<SharedString>),
    ToggleIntegration {
        app: SharedString,
        enabled: bool,
    },
    OpenArtifact(SharedString),
}

impl EventEmitter<AssistantHomeEvent> for AssistantHomeScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenMenu {
    Add,
    Approval,
    Model,
    Folder,
    Apps,
}

/// AI assistant start screen (`assistant-home`) with greeting, tool composer, and starter chips.
pub struct AssistantHomeScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    greeting: Option<SharedString>,
    loading: bool,
    empty: bool,
    composer_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    open_menu: Option<OpenMenu>,
    last_menu: Cell<OpenMenu>,
    hovered_chip: Option<SharedString>,
    approval_mode: AssistantApprovalMode,
    models: Vec<AssistantModelOption>,
    selected_model: usize,
    folders: Vec<SharedString>,
    active_folder: Option<SharedString>,
    connected_apps: Vec<(SharedString, bool)>,
    web_search_enabled: bool,
    extended_thinking: bool,
    attachments: Vec<AssistantAttachment>,
    chips: Vec<StarterPromptChip>,
    turns: Vec<AssistantTurn>,
    expanded_traces: Vec<SharedString>,
    recording: bool,
    recording_started: Option<Instant>,
    stream_state: Option<(SharedString, SharedString, Instant)>,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
}

fn default_chat_sidebar_groups(cx: &App) -> Vec<ShellNavGroup> {
    vec![
        ShellNavGroup::primary(vec![
            ShellNavItem::new("new-chat", tr(cx, "New chat", "新建对话"), ProIcon::NewChat),
            ShellNavItem::new("folders", tr(cx, "Folders", "文件夹"), ProIcon::Folder),
            ShellNavItem::new(
                "integrations",
                tr(cx, "Integrations", "集成"),
                ProIcon::Grid,
            ),
        ]),
        ShellNavGroup::named(
            tr(cx, "Pinned", "已置顶"),
            vec![ShellNavItem::new(
                "pinned-q3",
                "Q3 launch messaging",
                ProIcon::Pin,
            )],
        ),
        ShellNavGroup::named(
            tr(cx, "Projects", "项目"),
            vec![
                ShellNavItem::new("q3", "Q3 Campaign", ProIcon::ProjectFolder),
                ShellNavItem::new(
                    "q3-hooks",
                    "Ad hooks for the linen overshirt",
                    ProIcon::Thread,
                )
                .with_unread(true),
                ShellNavItem::new("q3-budget", "Q3 budget split by channel", ProIcon::Thread),
                ShellNavItem::new("brand", "Brand assets", ProIcon::Folder),
                ShellNavItem::new("hiring", "Hiring", ProIcon::Folder),
                ShellNavItem::new("research", "Research", ProIcon::Folder),
            ],
        ),
        ShellNavGroup::named(
            tr(cx, "Chats", "对话"),
            vec![
                ShellNavItem::new("chat-onboarding", "Onboarding email draft", ProIcon::Thread),
                ShellNavItem::new("chat-calendar", "August content calendar", ProIcon::Thread),
            ],
        ),
    ]
}

fn default_models() -> Vec<AssistantModelOption> {
    vec![
        AssistantModelOption::new("auto", "Auto", None, "Best model for each task"),
        AssistantModelOption::new(
            "claude-sonnet-4-5",
            "Claude Sonnet 4.5",
            Some("Claude"),
            "Everyday writing & code",
        ),
        AssistantModelOption::new(
            "claude-opus-4-1",
            "Claude Opus 4.1",
            Some("Claude"),
            "Deep reasoning & synthesis",
        ),
        AssistantModelOption::new(
            "gpt-5",
            "GPT-5",
            Some("OpenAI"),
            "Fast multimodal reasoning",
        ),
        AssistantModelOption::new("o3", "o3", Some("OpenAI"), "Step-by-step analysis"),
        AssistantModelOption::new(
            "gemini-2-5-pro",
            "Gemini 2.5 Pro",
            Some("Gemini"),
            "Long context & workspace docs",
        ),
        AssistantModelOption::new(
            "deepseek-r1",
            "DeepSeek R1",
            Some("DeepSeek"),
            "Structured verification",
        ),
    ]
}

fn default_starter_chips() -> Vec<StarterPromptChip> {
    vec![
        StarterPromptChip::new(
            "company-knowledge",
            "Company knowledge",
            "Summarize our Q3 launch messaging and approved positioning across connected docs.",
            StarterChipKind::CompanyKnowledge,
        ),
        StarterPromptChip::new(
            "create-document",
            "Create document",
            "Draft a crisp Q3 launch plan with owners, rollout milestones, and success metrics.",
            StarterChipKind::CreateDocument,
        ),
        StarterPromptChip::new(
            "create-presentation",
            "Create presentation",
            "Outline an 8-slide Q3 campaign kickoff deck for marketing and product leads.",
            StarterChipKind::CreatePresentation,
        ),
        StarterPromptChip::new(
            "create-table",
            "Create table",
            "Build a channel budget table comparing planned vs. committed Q3 spend.",
            StarterChipKind::CreateTable,
        ),
        StarterPromptChip::new(
            "generate-image",
            "Generate image",
            "Create three clean hero visual directions for the autumn linen overshirt campaign.",
            StarterChipKind::GenerateImage,
        ),
        StarterPromptChip::new(
            "deep-research",
            "Deep research",
            "Research competitor pricing tiers and enterprise packaging changes over the past quarter.",
            StarterChipKind::DeepResearch,
        ),
        StarterPromptChip::new(
            "visualize-data",
            "Visualize data",
            "Chart weekly conversion lift across paid social, search, and partner co-marketing.",
            StarterChipKind::VisualizeData,
        ),
    ]
}

fn synthesize_reply(
    prompt: &str,
) -> (
    SharedString,
    Vec<SharedString>,
    String,
    Option<GeneratedArtifactChip>,
) {
    let lower = prompt.to_ascii_lowercase();
    if lower.contains("table") || lower.contains("budget") || lower.contains("spend") {
        (
            "Checked 3 workspace sheets · 1.1s".into(),
            vec![
                "Read Q3 budget split by channel in Q3 Campaign".into(),
                "Reconciled committed vendor quotes from Slack #finance".into(),
                "Generated budget-by-channel.csv with formula totals".into(),
            ],
            "Here is the reconciled Q3 channel spend breakdown. Paid social and partner co-marketing stay within target, leaving $1,350 unallocated buffer for launch week.".to_owned(),
            Some(GeneratedArtifactChip {
                name: "budget-by-channel.csv".into(),
                is_sheet: true,
            }),
        )
    } else if lower.contains("research") || lower.contains("competitor") {
        (
            "Searched 6 sources & workspace notes · 1.8s".into(),
            vec![
                "Reviewed Research folder benchmarks".into(),
                "Compared seat-based vs. usage-credit tiers across 5 peers".into(),
                "Compiled competitor-pricing.csv matrix".into(),
            ],
            "Across five benchmarked workspaces, hybrid seat + pooled credit plans grew adoption fastest while keeping predictable monthly caps for teams.".to_owned(),
            Some(GeneratedArtifactChip {
                name: "competitor-pricing.csv".into(),
                is_sheet: true,
            }),
        )
    } else {
        (
            "Searched 4 workspace docs · 1.2s".into(),
            vec![
                "Read Q3 launch messaging and Brand assets".into(),
                "Pulled timeline owners from Linear launch checklist".into(),
                "Drafted q3-launch-plan.md".into(),
            ],
            "I put together a structured first draft aligned with your Q3 launch messaging: Thursday 09:00 UTC target window, verified webhook retry gates, and partner co-marketing sign-off.".to_owned(),
            Some(GeneratedArtifactChip {
                name: "q3-launch-plan.md".into(),
                is_sheet: false,
            }),
        )
    }
}

impl AssistantHomeScreen {
    /// Construct a new [`AssistantHomeScreen`] hosted in [`AppShell`] (`Chat` preset, `"Acme"`).
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle().tab_stop(true);
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Chat, shell_cx)
                .with_groups(default_chat_sidebar_groups(shell_cx))
                .with_active(tr(shell_cx, "New chat", "新建对话"))
                .with_work_area(true, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                let lower = label.to_ascii_lowercase();
                if lower == "new chat" || label.as_ref() == "新建对话" {
                    this.reset_conversation(cx);
                }
                cx.emit(AssistantHomeEvent::Navigate(label.clone()));
            }
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            focus,
            shell,
            standalone: false,
            greeting: None,
            loading: false,
            empty: false,
            composer_editor: None,
            editor_focused: false,
            open_menu: None,
            last_menu: Cell::new(OpenMenu::Model),
            hovered_chip: None,
            approval_mode: AssistantApprovalMode::RequestApproval,
            models: default_models(),
            selected_model: 0,
            folders: vec![
                "Q3 Campaign".into(),
                "Brand assets".into(),
                "Hiring".into(),
                "Research".into(),
            ],
            active_folder: None,
            connected_apps: vec![
                ("Gmail".into(), true),
                ("Slack".into(), true),
                ("ClickUp".into(), true),
                ("Linear".into(), false),
                ("Notion".into(), false),
            ],
            web_search_enabled: true,
            extended_thinking: false,
            attachments: Vec::new(),
            chips: default_starter_chips(),
            turns: Vec::new(),
            expanded_traces: Vec::new(),
            recording: false,
            recording_started: None,
            stream_state: None,
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(820.)),
        }
    }

    /// Render without the outer [`AppShell`] frame when embedded in a custom host.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace name displayed in the sidebar header.
    pub fn with_workspace(
        self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws: SharedString = workspace.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_workspace(ws);
            cx.notify();
        });
        self
    }

    /// Override the signed-in account shown at the bottom of the sidebar.
    pub fn with_user(
        self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let n: SharedString = name.into();
        let e: SharedString = email.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_user(n, e);
            cx.notify();
        });
        self
    }

    /// Override the headline greeting above the prompt composer.
    pub fn with_greeting(mut self, greeting: impl Into<SharedString>) -> Self {
        self.greeting = Some(greeting.into());
        self
    }

    /// Replace the default starter prompt chips below the composer.
    pub fn with_chips(mut self, chips: Vec<StarterPromptChip>) -> Self {
        self.chips = chips;
        self
    }

    /// Replace the selectable models in the composer model menu.
    pub fn with_models(mut self, models: Vec<AssistantModelOption>) -> Self {
        if !models.is_empty() {
            self.models = models;
            self.selected_model = 0;
        }
        self
    }

    /// Start in skeleton loading mode while connected apps and quick prompts load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Start with no quick-prompt suggestions configured (`empty` state).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle the skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle the empty suggestions state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the currently selected model identifier.
    pub fn selected_model_id(&self) -> SharedString {
        self.models
            .get(self.selected_model)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "auto".into())
    }

    /// Return the active conversation turns.
    pub fn turns(&self) -> &[AssistantTurn] {
        &self.turns
    }

    /// Reset an active conversation back to the centered start screen.
    pub fn reset_conversation(&mut self, cx: &mut Context<Self>) {
        self.turns.clear();
        self.expanded_traces.clear();
        self.stream_state = None;
        self.open_menu = None;
        cx.notify();
    }

    /// Submit a prompt string, transitioning from the start view into the active conversation.
    pub fn submit_prompt(&mut self, raw: impl Into<SharedString>, cx: &mut Context<Self>) -> bool {
        if self.loading {
            return false;
        }
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() && self.attachments.is_empty() {
            return false;
        }
        let prompt_text: SharedString = if trimmed.is_empty() {
            "Review attached workspace files.".into()
        } else {
            trimmed.clone().into()
        };
        let staged = std::mem::take(&mut self.attachments);
        let model_id = self.selected_model_id();
        let folder = self.active_folder.clone();
        let user_id: SharedString = format!("u-{}", self.turns.len() + 1).into();
        let asst_id: SharedString = format!("a-{}", self.turns.len() + 2).into();
        let (trace_summary, trace_steps, reply_full, artifact) = synthesize_reply(&prompt_text);

        let reduced = Theme::global(cx).reduced_motion;
        let initial_text: SharedString = if reduced {
            reply_full.clone().into()
        } else {
            reply_full.chars().take(12).collect::<String>().into()
        };

        self.turns.push(AssistantTurn::User {
            id: user_id,
            text: prompt_text.clone(),
            folder: folder.clone(),
            attachments: staged.clone(),
        });
        self.turns.push(AssistantTurn::Assistant {
            id: asst_id.clone(),
            trace_summary,
            trace_steps,
            text: initial_text,
            artifact,
            streaming: !reduced,
        });
        self.open_menu = None;
        self.empty = false;
        if !reduced {
            self.stream_state = Some((asst_id, reply_full.into(), Instant::now()));
        }

        cx.emit(AssistantHomeEvent::SubmitPrompt {
            prompt: prompt_text,
            model_id,
            approval: self.approval_mode,
            folder,
            attachments: staged,
        });
        cx.notify();
        true
    }

    fn stop_streaming(&mut self, cx: &mut Context<Self>) {
        if let Some((aid, _, _)) = self.stream_state.take() {
            for turn in &mut self.turns {
                if let AssistantTurn::Assistant { id, streaming, .. } = turn
                    && id == &aid
                {
                    *streaming = false;
                }
            }
            cx.notify();
        }
    }

    fn is_streaming(&self) -> bool {
        self.stream_state.is_some()
    }

    fn advance_stream(&mut self, w: &mut Window, reduced: bool) {
        let Some((msg_id, full_text, started_at)) = self.stream_state.clone() else {
            return;
        };
        let elapsed_ms = started_at.elapsed().as_secs_f32() * 1000.;
        let ratio = if reduced {
            1.0
        } else {
            (elapsed_ms / 680.).clamp(0.08, 1.0)
        };
        let chars: Vec<char> = full_text.chars().collect();
        let take_n = ((chars.len() as f32 * ratio).ceil() as usize).min(chars.len());
        let prefix: String = chars.into_iter().take(take_n).collect();
        let done_now = ratio >= 0.999;

        for turn in &mut self.turns {
            if let AssistantTurn::Assistant {
                id,
                text,
                streaming,
                ..
            } = turn
                && id == &msg_id
            {
                *text = prefix.clone().into();
                if done_now {
                    *streaming = false;
                }
            }
        }
        if done_now {
            self.stream_state = None;
        } else {
            w.request_animation_frame();
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString =
                tr(cx, "Help me brainstorm ideas for…", "帮我构思关于…的想法").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::InlineMulti, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_entity, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    this.open_menu = None;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => {
                    let val = ed_entity.read(cx).value().to_string();
                    if this.submit_prompt(val, cx) {
                        ed_entity.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    if this.recording {
                        this.recording = false;
                        this.recording_started = None;
                        cx.notify();
                    } else if this.open_menu.take().is_some() {
                        cx.notify();
                    } else if this.is_streaming() {
                        this.stop_streaming(cx);
                    }
                }
                _ => {}
            })
            .detach();
            self.composer_editor = Some(ed);
        }
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.editor_focused || ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control
        {
            return;
        }
        let k = ev.keystroke.key.as_str();
        if k == "/" {
            if let Some(ed) = &self.composer_editor {
                {
                    let fh = ed.read(cx).focus.clone();
                    fh.focus(w, cx);
                }
            }
            return;
        }
        if k == "escape" {
            if self.open_menu.take().is_some() {
                cx.notify();
            } else if self.recording {
                self.recording = false;
                self.recording_started = None;
                cx.notify();
            } else if self.is_streaming() {
                self.stop_streaming(cx);
            }
            return;
        }
        if self.turns.is_empty()
            && !self.loading
            && !self.empty
            && let Ok(idx) = k.parse::<usize>()
            && (1..=self.chips.len()).contains(&idx)
            && let Some(chip) = self.chips.get(idx - 1).cloned()
        {
            cx.emit(AssistantHomeEvent::SelectStarter(chip.id));
            self.submit_prompt(chip.prompt, cx);
        }
    }

    fn render_popover_menu(&self, menu: OpenMenu, cx: &Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let card_surface: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };

        match menu {
            OpenMenu::Add => div()
                .id(pro_eid(&self.id, "menu-add"))
                .role(Role::Menu)
                .w(px(256.))
                .p(px(4.))
                .rounded(px(12.))
                .bg(card_surface)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .px(px(8.))
                        .pt(px(5.))
                        .pb(px(2.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "Add", "添加")),
                )
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "add-opt-file"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Paperclip, 15., p.muted))
                        .child(tr(cx, "Attach files or photos", "上传文件或图片")),
                    cx,
                    |this, _, cx| {
                        this.attachments
                            .push(AssistantAttachment::new("q3-campaign-brief.pdf", "412 KB"));
                        this.open_menu = None;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "add-opt-folder"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Folder, 15., p.muted))
                        .child(tr(cx, "Add project folder", "关联项目文件夹")),
                    cx,
                    |this, _, cx| {
                        this.active_folder = Some("Q3 Campaign".into());
                        this.open_menu = None;
                        cx.emit(AssistantHomeEvent::SelectFolder(Some("Q3 Campaign".into())));
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "add-opt-web"))
                        .role(Role::CheckBox)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Globe, 15., p.muted))
                        .child(div().flex_1().child(tr(cx, "Web search", "联网搜索")))
                        .when(self.web_search_enabled, |d| {
                            d.child(pro_icon(ProIcon::Check, 14., p.ink))
                        }),
                    cx,
                    |this, _, cx| {
                        this.web_search_enabled = !this.web_search_enabled;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "add-opt-think"))
                        .role(Role::CheckBox)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Sparkles, 15., p.muted))
                        .child(
                            div()
                                .flex_1()
                                .child(tr(cx, "Extended thinking", "深度思考")),
                        )
                        .when(self.extended_thinking, |d| {
                            d.child(pro_icon(ProIcon::Check, 14., p.ink))
                        }),
                    cx,
                    |this, _, cx| {
                        this.extended_thinking = !this.extended_thinking;
                        cx.notify();
                    },
                ))
                .child(div().my(px(3.)).mx(px(6.)).h(px(1.)).bg(p.line))
                .child(
                    div()
                        .px(px(8.))
                        .pt(px(3.))
                        .pb(px(2.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "Plugins", "插件")),
                )
                .children(
                    [
                        ("Claude", "Artifacts canvas"),
                        ("Notion", "Workspace pages"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(pi, (provider, label))| {
                        let label_owned = label.to_owned();
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("add-plugin-{pi}")))
                                .role(Role::MenuItem)
                                .tab_index(0)
                                .h(px(32.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .child(pro_app_tile(provider, 16., cx))
                                .child(label),
                            cx,
                            move |this, _, cx| {
                                this.attachments
                                    .push(AssistantAttachment::new(label_owned.clone(), "Plugin"));
                                this.open_menu = None;
                                cx.notify();
                            },
                        )
                    }),
                ),
            OpenMenu::Approval => {
                let appr_sensor = pro_selection_sensor();
                div()
                    .id(pro_eid(&self.id, "menu-approval"))
                    .role(Role::ListBox)
                    .w(px(264.))
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(card_surface)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .px(px(8.))
                            .pt(px(5.))
                            .pb(px(4.))
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(tr(
                                cx,
                                "How should actions be approved?",
                                "如何审批外部操作？",
                            ))
                            .child(div().text_color(p.muted).child(tr(
                                cx,
                                "Learn more",
                                "了解更多",
                            ))),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(pro_pill_indicator(
                                pro_eid(&self.id, "appr-pill"),
                                format!("{:?}", self.approval_mode),
                                appr_sensor.clone(),
                                6.,
                                p.chip,
                                None,
                                false,
                            ))
                            .children(
                                [
                                    (
                                        AssistantApprovalMode::RequestApproval,
                                        tr(cx, "Request approval", "请求审批"),
                                        tr(
                                            cx,
                                            "Ask before writes or external actions",
                                            "写入或外部操作前询问",
                                        ),
                                        false,
                                    ),
                                    (
                                        AssistantApprovalMode::AutoApproveSafe,
                                        tr(cx, "Auto-approve safe", "只读自动批准"),
                                        tr(
                                            cx,
                                            "Read workspace docs without pausing",
                                            "安全读取文档无需暂停",
                                        ),
                                        false,
                                    ),
                                    (
                                        AssistantApprovalMode::FullAccess,
                                        tr(cx, "Full access", "完全自动执行"),
                                        tr(
                                            cx,
                                            "Execute actions without confirmation",
                                            "无需确认直接执行",
                                        ),
                                        true,
                                    ),
                                ]
                                .into_iter()
                                .enumerate()
                                .map(
                                    |(ai, (mode, name, desc, is_alert))| {
                                        let selected = self.approval_mode == mode;
                                        let item_fg = if is_alert { p.red_ink } else { p.ink };
                                        let icon_el = if is_alert {
                                            pro_svg(
                                                "0 0 66.543 66.543",
                                                SHIELD_ALERT_SVG,
                                                15.,
                                                p.red_ink,
                                            )
                                        } else if mode == AssistantApprovalMode::RequestApproval {
                                            pro_svg(
                                                "0 0 68.155 68.155",
                                                HAND_APPROVAL_SVG,
                                                15.,
                                                p.muted,
                                            )
                                        } else {
                                            pro_icon(ProIcon::Check, 15., p.muted)
                                        };
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("appr-opt-{ai}")))
                                                .role(Role::ListBoxOption)
                                                .tab_index(0)
                                                .relative()
                                                .px(px(8.))
                                                .py(px(6.))
                                                .rounded(px(6.))
                                                .when(selected, |d| {
                                                    d.child(pro_mark_selected(&appr_sensor))
                                                })
                                                .when(!selected, |d| {
                                                    d.hover(move |s| s.bg(p.hover))
                                                })
                                                .flex()
                                                .items_start()
                                                .gap(px(10.))
                                                .cursor_pointer()
                                                .child(div().mt(px(2.)).child(icon_el))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .flex()
                                                        .flex_col()
                                                        .child(
                                                            div()
                                                                .text_size(px(13.))
                                                                .font_weight(if selected {
                                                                    FontWeight::MEDIUM
                                                                } else {
                                                                    FontWeight::NORMAL
                                                                })
                                                                .text_color(item_fg)
                                                                .child(name),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(12.))
                                                                .text_color(p.muted)
                                                                .child(desc),
                                                        ),
                                                )
                                                .when(selected, |d| {
                                                    d.child(div().mt(px(2.)).child(pro_icon(
                                                        ProIcon::Check,
                                                        14.,
                                                        p.ink,
                                                    )))
                                                }),
                                            cx,
                                            move |this, _, cx| {
                                                this.approval_mode = mode;
                                                this.open_menu = None;
                                                cx.notify();
                                            },
                                        )
                                    },
                                ),
                            ),
                    )
            }
            OpenMenu::Model => {
                let mdl_sensor = pro_selection_sensor();
                div()
                    .id(pro_eid(&self.id, "menu-model"))
                    .role(Role::ListBox)
                    .relative()
                    .w(px(260.))
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(card_surface)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "mdl-pill"),
                        format!("mdl-{}", self.selected_model),
                        mdl_sensor.clone(),
                        6.,
                        p.chip,
                        None,
                        false,
                    ))
                    .children(self.models.iter().enumerate().map(|(mi, mdl)| {
                        let selected = mi == self.selected_model;
                        let mid = mdl.id.clone();
                        let lead = if let Some(prov) = &mdl.provider {
                            pro_app_artwork(prov, 15., cx)
                                .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 15., p.muted))
                        } else {
                            pro_icon(ProIcon::Sparkles, 15., p.muted)
                        };
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("mdl-opt-{mi}")))
                                .role(Role::ListBoxOption)
                                .tab_index(0)
                                .relative()
                                .px(px(8.))
                                .py(px(6.))
                                .rounded(px(6.))
                                .when(selected, |d| d.child(pro_mark_selected(&mdl_sensor)))
                                .when(!selected, |d| d.hover(move |s| s.bg(p.hover)))
                                .flex()
                                .items_start()
                                .gap(px(10.))
                                .cursor_pointer()
                                .child(div().mt(px(2.)).child(lead))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(if selected {
                                                    FontWeight::MEDIUM
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .text_color(if selected { p.strong } else { p.ink })
                                                .truncate()
                                                .child(mdl.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .truncate()
                                                .child(mdl.summary.clone()),
                                        ),
                                )
                                .when(selected, |d| {
                                    d.child(div().mt(px(2.)).child(pro_icon(
                                        ProIcon::Check,
                                        14.,
                                        p.ink,
                                    )))
                                }),
                            cx,
                            move |this, _, cx| {
                                this.selected_model = mi;
                                this.open_menu = None;
                                cx.emit(AssistantHomeEvent::SelectModel(mid.clone()));
                                cx.notify();
                            },
                        )
                    }))
            }
            OpenMenu::Folder => {
                let fld_sensor = pro_selection_sensor();
                let mut rows =
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(pro_pill_indicator(
                            pro_eid(&self.id, "fld-pill"),
                            self.active_folder.clone().unwrap_or_else(|| "none".into()),
                            fld_sensor.clone(),
                            6.,
                            p.chip,
                            None,
                            false,
                        ));
                for (fi, fname) in self.folders.iter().enumerate() {
                    let chosen = self.active_folder.as_ref() == Some(fname);
                    let f_clone = fname.clone();
                    rows = rows.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("fld-opt-{fi}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .relative()
                            .h(px(32.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .when(chosen, |d| d.child(pro_mark_selected(&fld_sensor)))
                            .when(!chosen, |d| d.hover(move |s| s.bg(p.hover)))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(tray_folder_icon(15.))
                            .child(div().flex_1().truncate().child(fname.clone()))
                            .when(chosen, |d| d.child(pro_icon(ProIcon::Check, 14., p.ink))),
                        cx,
                        move |this, _, cx| {
                            let next = if this.active_folder.as_ref() == Some(&f_clone) {
                                None
                            } else {
                                Some(f_clone.clone())
                            };
                            this.active_folder = next.clone();
                            this.open_menu = None;
                            cx.emit(AssistantHomeEvent::SelectFolder(next));
                            cx.notify();
                        },
                    ));
                }
                div()
                    .id(pro_eid(&self.id, "menu-folder"))
                    .role(Role::Menu)
                    .w(px(224.))
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(card_surface)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .px(px(8.))
                            .pt(px(5.))
                            .pb(px(3.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Project folders", "项目文件夹")),
                    )
                    .child(rows)
            }
            OpenMenu::Apps => {
                let mut panel = div()
                    .id(pro_eid(&self.id, "menu-apps"))
                    .role(Role::Menu)
                    .w(px(244.))
                    .p(px(5.))
                    .rounded(px(10.))
                    .bg(card_surface)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .px(px(8.))
                            .pt(px(4.))
                            .pb(px(3.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Connected apps", "已连接应用")),
                    );
                for (ai, (app_name, enabled)) in self.connected_apps.iter().enumerate() {
                    let app_str = app_name.clone();
                    let next_on = !*enabled;
                    let lead = if app_name.as_ref() == "ClickUp" {
                        div()
                            .size(px(18.))
                            .rounded(px(5.))
                            .bg(if p.dark { rgb(0x222226) } else { rgb(0xffffff) })
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(clickup_mark(12.))
                            .into_any_element()
                    } else {
                        pro_app_tile(app_name, 18., cx)
                    };
                    panel = panel.child(
                        div()
                            .h(px(34.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .child(lead)
                                    .child(app_name.clone()),
                            )
                            .child(pro_switch(
                                pro_eid(&self.id, format!("app-sw-{ai}")),
                                app_name.clone(),
                                *enabled,
                                cx,
                                move |this, _, cx| {
                                    if let Some((_, flag)) =
                                        this.connected_apps.iter_mut().find(|(n, _)| n == &app_str)
                                    {
                                        *flag = next_on;
                                        cx.emit(AssistantHomeEvent::ToggleIntegration {
                                            app: app_str.clone(),
                                            enabled: next_on,
                                        });
                                        cx.notify();
                                    }
                                },
                            )),
                    );
                }
                panel
                    .child(div().my(px(3.)).mx(px(6.)).h(px(1.)).bg(p.line))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "apps-manage"))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(12.5))
                            .text_color(p.muted)
                            .cursor_pointer()
                            .child(tr(cx, "Manage integrations", "管理全部集成"))
                            .child(pro_icon(ProIcon::ChevronRight, 13., p.faint)),
                        cx,
                        |this, _, cx| {
                            this.open_menu = None;
                            cx.emit(AssistantHomeEvent::Navigate("Integrations".into()));
                            cx.notify();
                        },
                    ))
            }
        }
    }

    fn render_composer(
        &self,
        compact_composer: bool,
        elapsed_ms: f32,
        skel_phase: f32,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let tray_bg: Hsla = if p.dark {
            rgb(0x161618).into()
        } else {
            rgb(0xf5f5f5).into()
        };
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let draft = self
            .composer_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        let can_send = !self.loading && (!draft.is_empty() || !self.attachments.is_empty());
        let streaming = self.is_streaming();
        let is_alert = self.approval_mode == AssistantApprovalMode::FullAccess;
        let appr_fg = if is_alert { p.red_ink } else { p.muted };

        let cur_model = self
            .models
            .get(self.selected_model)
            .cloned()
            .unwrap_or_else(|| AssistantModelOption::new("auto", "Auto", None, ""));

        let right_bar = if self.recording {
            let rec_secs = self
                .recording_started
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(div().h(px(20.)).flex().items_center().gap(px(3.)).children(
                    (0..9usize).map(|bi| {
                        let h = if reduced {
                            8.
                        } else {
                            4. + 12. * ((elapsed_ms / 170. + bi as f32 * 0.65).sin().abs())
                        };
                        div().w(px(2.)).h(px(h)).rounded(px(1.)).bg(p.blue)
                    }),
                ))
                .child(
                    div()
                        .mr(px(2.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(format!("0:{rec_secs:02}")),
                )
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "rec-cancel"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Cancel dictation", "取消语音输入"))
                        .tab_index(0)
                        .size(px(28.))
                        .rounded(px(8.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Close, 14., p.ink)),
                    cx,
                    |this, _, cx| {
                        this.recording = false;
                        this.recording_started = None;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "rec-commit"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Use dictation", "使用语音文本"))
                        .tab_index(0)
                        .size(px(28.))
                        .rounded(px(8.))
                        .bg(p.primary)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Check, 15., p.on_primary)),
                    cx,
                    |this, w, cx| {
                        this.recording = false;
                        this.recording_started = None;
                        if let Some(ed) = this.composer_editor.clone() {
                            ed.update(cx, |e, cx| {
                                e.set_value(
                                    "Draft a Q3 launch checklist with channel owners and dates",
                                    w,
                                    cx,
                                );
                            });
                        }
                        cx.notify();
                    },
                ))
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "composer-model"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Select a model", "选择模型"))
                        .tab_index(0)
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(6.))
                        .rounded(px(8.))
                        .when(self.open_menu == Some(OpenMenu::Model), |d| d.bg(p.hover))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .when_some(cur_model.provider.as_ref(), |d, prov| {
                            if let Some(mark) = pro_app_artwork(prov, 14., cx) {
                                d.child(mark)
                            } else {
                                d
                            }
                        })
                        .child(div().max_w(px(150.)).truncate().child(cur_model.name))
                        .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                    cx,
                    |this, _, cx| {
                        this.open_menu = if this.open_menu == Some(OpenMenu::Model) {
                            None
                        } else {
                            Some(OpenMenu::Model)
                        };
                        cx.notify();
                    },
                ))
                .child(pro_icon_btn(
                    pro_eid(&self.id, "composer-mic"),
                    tr(cx, "Voice dictation", "语音输入"),
                    ProIcon::Mic,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.recording = true;
                        this.recording_started = Some(Instant::now());
                        cx.notify();
                    },
                ))
                .child(if streaming {
                    activate(
                        div()
                            .id(pro_eid(&self.id, "composer-stop"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Stop generation", "停止生成"))
                            .tab_index(0)
                            .size(px(28.))
                            .ml(px(2.))
                            .rounded(px(8.))
                            .bg(p.chip_hover)
                            .hover(move |s| s.bg(p.line_strong))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(div().size(px(9.)).rounded(px(2.)).bg(p.ink)),
                        cx,
                        |this, _, cx| this.stop_streaming(cx),
                    )
                } else {
                    let send_bg: Hsla = if can_send {
                        p.primary
                    } else if p.dark {
                        hsla(0., 0., 1., 0.08)
                    } else {
                        hsla(0., 0., 0., 0.06)
                    };
                    activate(
                        div()
                            .id(pro_eid(&self.id, "composer-send"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Send message", "发送消息"))
                            .tab_index(0)
                            .size(px(28.))
                            .ml(px(2.))
                            .rounded(px(8.))
                            .bg(send_bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(can_send, |d| d.cursor_pointer())
                            .child(pro_icon(
                                ProIcon::ArrowUp,
                                15.,
                                if can_send { p.on_primary } else { p.faint },
                            )),
                        cx,
                        |this, w, cx| {
                            if let Some(ed) = this.composer_editor.clone() {
                                let val = ed.read(cx).value().to_string();
                                if this.submit_prompt(val, cx) {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                            }
                        },
                    )
                })
        };

        let att_strip = div()
            .px(px(4.))
            .pb(px(8.))
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .children(self.attachments.iter().enumerate().map(|(ai, att)| {
                div()
                    .h(px(28.))
                    .pl(px(9.))
                    .pr(px(4.))
                    .rounded(px(8.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.5))
                    .text_color(p.ink)
                    .child(pro_icon(ProIcon::Paperclip, 13., p.muted))
                    .child(att.name.clone())
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("rm-att-{ai}")),
                        tr(cx, "Remove attachment", "移除附件"),
                        ProIcon::Close,
                        20.,
                        false,
                        cx,
                        move |this, _, cx| {
                            if ai < this.attachments.len() {
                                this.attachments.remove(ai);
                                cx.notify();
                            }
                        },
                    ))
            }));

        let input_content = if self.recording {
            div()
                .px(px(4.))
                .h(px(40.))
                .flex()
                .items_start()
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "Listening… Draft a Q3 launch checklist with channel owners…",
                    "正在聆听… 起草一份包含渠道负责人与时间节点的 Q3 发布清单…",
                ))
        } else {
            div()
                .px(px(4.))
                .h(px(40.))
                .when_some(self.composer_editor.clone(), |d, ed| d.child(ed))
        };

        let write_card = div()
            .w_full()
            .pt(px(14.))
            .px(px(10.))
            .pb(px(10.))
            .rounded(px(15.))
            .bg(card_bg)
            .border_1()
            .border_color(if self.editor_focused {
                p.line_strong
            } else {
                p.line
            })
            .flex()
            .flex_col()
            .child(pro_fold(
                pro_eid(&self.id, "composer-att-fold"),
                !self.attachments.is_empty(),
                36.,
                0.,
                move || att_strip.into_any_element(),
            ))
            .child(pro_view_transition(
                pro_eid(&self.id, "composer-input-vtx"),
                if self.recording { "rec" } else { "edit" },
                input_content,
            ))
            .child(
                div()
                    .mt(px(14.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "composer-add"),
                                tr(cx, "Add", "添加"),
                                ProIcon::Plus,
                                28.,
                                self.open_menu == Some(OpenMenu::Add),
                                cx,
                                |this, _, cx| {
                                    this.open_menu = if this.open_menu == Some(OpenMenu::Add) {
                                        None
                                    } else {
                                        Some(OpenMenu::Add)
                                    };
                                    cx.notify();
                                },
                            ))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "composer-approval"))
                                    .role(Role::Button)
                                    .aria_label(self.approval_mode.label(cx))
                                    .tab_index(0)
                                    .h(px(28.))
                                    .px(px(if compact_composer { 6. } else { 8. }))
                                    .rounded(px(8.))
                                    .when(self.open_menu == Some(OpenMenu::Approval), |d| {
                                        d.bg(if is_alert { p.red_soft } else { p.hover })
                                    })
                                    .hover(move |s| {
                                        s.bg(if is_alert { p.red_soft } else { p.hover })
                                    })
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(13.))
                                    .text_color(appr_fg)
                                    .cursor_pointer()
                                    .child(pro_svg(
                                        "0 0 68.155 68.155",
                                        HAND_APPROVAL_SVG,
                                        15.,
                                        appr_fg,
                                    ))
                                    .when(!compact_composer, |d| {
                                        d.child(self.approval_mode.label(cx))
                                    }),
                                cx,
                                |this, _, cx| {
                                    this.open_menu = if this.open_menu == Some(OpenMenu::Approval) {
                                        None
                                    } else {
                                        Some(OpenMenu::Approval)
                                    };
                                    cx.notify();
                                },
                            )),
                    )
                    .child(right_bar),
            );

        let folder_label: SharedString = self
            .active_folder
            .clone()
            .unwrap_or_else(|| tr(cx, "Work in a folder", "在文件夹中协作").into());
        let has_folder = self.active_folder.is_some();

        let tray_apps = if self.loading {
            div()
                .h(px(26.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(pro_skeleton_bar(72., 11., 5., skel_phase, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(pro_skeleton_bar(13., 13., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(13., 13., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(13., 13., 4., skel_phase, cx)),
                )
                .into_any_element()
        } else {
            let mut logos = div().flex().items_center().gap(px(6.));
            if let Some(gm) = pro_app_artwork("gmail", 13., cx) {
                logos = logos.child(gm);
            }
            if let Some(sl) = pro_app_artwork("slack", 13., cx) {
                logos = logos.child(sl);
            }
            logos = logos.child(clickup_mark(13.));

            activate(
                div()
                    .id(pro_eid(&self.id, "tray-apps"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(26.))
                    .pl(px(9.))
                    .pr(px(7.))
                    .rounded_full()
                    .when(self.open_menu == Some(OpenMenu::Apps), |d| d.bg(p.hover))
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child(tr(cx, "Connect apps", "连接应用"))
                    .child(logos),
                cx,
                |this, _, cx| {
                    this.open_menu = if this.open_menu == Some(OpenMenu::Apps) {
                        None
                    } else {
                        Some(OpenMenu::Apps)
                    };
                    cx.notify();
                },
            )
            .into_any_element()
        };

        let tray_row = div()
            .h(px(36.))
            .px(px(6.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "tray-folder"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(26.))
                    .pl(px(7.))
                    .pr(px(9.))
                    .rounded_full()
                    .when(self.open_menu == Some(OpenMenu::Folder), |d| d.bg(p.hover))
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.))
                    .text_color(if has_folder { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child(tray_folder_icon(16.))
                    .child(div().max_w(px(160.)).truncate().child(folder_label)),
                cx,
                |this, _, cx| {
                    this.open_menu = if this.open_menu == Some(OpenMenu::Folder) {
                        None
                    } else {
                        Some(OpenMenu::Folder)
                    };
                    cx.notify();
                },
            ))
            .child(tray_apps);

        if let Some(m) = self.open_menu {
            self.last_menu.set(m);
        }
        let menu_kind = self.open_menu.unwrap_or_else(|| self.last_menu.get());
        let align_end = matches!(menu_kind, OpenMenu::Model | OpenMenu::Apps);
        let in_chat = !self.turns.is_empty();
        let menu_open = self.open_menu.is_some();
        let menu_key: SharedString = format!("{menu_kind:?}").into();
        let pop_vtx_id = pro_eid(&self.id, "composer-pop-vtx");
        let pop_el = self.render_popover_menu(menu_kind, cx);

        let mut wrap = div().relative().w_full().flex().flex_col().gap(px(6.));
        if in_chat {
            wrap = wrap.child(pro_presence(
                pro_eid(&self.id, "composer-pop-chat-pres"),
                menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .relative()
                        .top(px(5. * (1. - a)))
                        .opacity(a)
                        .w_full()
                        .flex()
                        .when(align_end, |d| d.justify_end())
                        .child(pro_view_transition(pop_vtx_id, menu_key, pop_el))
                        .into_any_element()
                },
            ));
        }
        wrap = wrap.child(
            div()
                .w_full()
                .rounded(px(16.))
                .bg(tray_bg)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(write_card)
                .child(tray_row),
        );
        if !in_chat {
            let pop_vtx_start_id = pro_eid(&self.id, "composer-pop-start-vtx");
            let menu_key_start: SharedString = format!("{menu_kind:?}").into();
            let pop_el_start = self.render_popover_menu(menu_kind, cx);
            wrap = wrap.child(pro_presence(
                pro_eid(&self.id, "composer-pop-start-pres"),
                menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .relative()
                        .top(px(-5. * (1. - a)))
                        .opacity(a)
                        .w_full()
                        .flex()
                        .when(align_end, |d| d.justify_end())
                        .child(pro_view_transition(
                            pop_vtx_start_id,
                            menu_key_start,
                            pop_el_start,
                        ))
                        .into_any_element()
                },
            ));
        }
        wrap
    }

    fn render_starter_chips(&self, start_w: f32, skel_phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let state_key: SharedString = if self.loading {
            "loading".into()
        } else if self.empty || self.chips.is_empty() {
            "empty".into()
        } else {
            "ready".into()
        };

        let inner: AnyElement = if self.loading {
            div()
                .mt(px(18.))
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(7.))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(134., 28., 14., skel_phase, cx))
                        .child(pro_skeleton_bar(118., 28., 14., skel_phase, cx))
                        .child(pro_skeleton_bar(130., 28., 14., skel_phase, cx))
                        .child(pro_skeleton_bar(96., 28., 14., skel_phase, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(112., 28., 14., skel_phase, cx))
                        .child(pro_skeleton_bar(106., 28., 14., skel_phase, cx))
                        .child(pro_skeleton_bar(102., 28., 14., skel_phase, cx)),
                )
                .into_any_element()
        } else if self.empty || self.chips.is_empty() {
            div()
                .mt(px(12.))
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No quick prompts yet", "暂无快捷提示"),
                    tr(
                        cx,
                        "Type any request above or restore the starter suggestions for documents, research, and charts.",
                        "在上方直接输入问题，或恢复文档、深度研究与图表快捷提示。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-restore-chips"),
                        tr(cx, "Show suggestions", "显示推荐提示").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.chips.is_empty() {
                                this.chips = default_starter_chips();
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let visible_limit = if start_w <= 460. {
                4
            } else if start_w <= 650. {
                6
            } else {
                self.chips.len()
            };
            let visible: Vec<StarterPromptChip> =
                self.chips.iter().take(visible_limit).cloned().collect();

            let chip_sensor = pro_selection_sensor();
            let hovered_key = self.hovered_chip.clone().unwrap_or_else(|| "none".into());
            let hovered_opt = self.hovered_chip.clone();

            let render_chip = |chip: StarterPromptChip| {
                let cid = chip.id.clone();
                let cid_hov = cid.clone();
                let prompt = chip.prompt.clone();
                let is_hovered = hovered_opt.as_ref() == Some(&cid);
                let sensor_for_chip = chip_sensor.clone();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("chip-{cid}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .relative()
                        .h(px(32.))
                        .pl(px(11.))
                        .pr(px(12.))
                        .rounded(px(16.))
                        .when(is_hovered, |d| d.child(pro_mark_selected(&sensor_for_chip)))
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                if this.hovered_chip.as_ref() != Some(&cid_hov) {
                                    this.hovered_chip = Some(cid_hov.clone());
                                    cx.notify();
                                }
                            } else if this.hovered_chip.as_ref() == Some(&cid_hov) {
                                this.hovered_chip = None;
                                cx.notify();
                            }
                        }))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(12.))
                        .text_color(if is_hovered { p.ink } else { p.text })
                        .cursor_pointer()
                        .child(pro_svg(
                            "0 0 24 24",
                            chip.kind.svg(),
                            15.,
                            if is_hovered { p.ink } else { p.text },
                        ))
                        .child(div().truncate().child(chip.label)),
                    cx,
                    move |this, _, cx| {
                        cx.emit(AssistantHomeEvent::SelectStarter(cid.clone()));
                        this.submit_prompt(prompt.clone(), cx);
                    },
                )
            };

            let pill_el = pro_pill_indicator(
                pro_eid(&self.id, "starter-chip-pill"),
                hovered_key,
                chip_sensor.clone(),
                16.,
                p.hover,
                None,
                false,
            );

            if start_w > 650. && visible.len() > 4 {
                let first_row: Vec<StarterPromptChip> = visible[..4].to_vec();
                let second_row: Vec<StarterPromptChip> = visible[4..].to_vec();
                div()
                    .relative()
                    .mt(px(18.))
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(7.))
                    .child(pill_el)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_center()
                            .gap(px(8.))
                            .children(first_row.into_iter().map(render_chip)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_center()
                            .gap(px(8.))
                            .children(second_row.into_iter().map(render_chip)),
                    )
                    .into_any_element()
            } else {
                div()
                    .relative()
                    .mt(px(14.))
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(px(6.))
                    .child(pill_el)
                    .children(visible.into_iter().map(render_chip))
                    .into_any_element()
            }
        };

        div().w_full().child(pro_view_transition(
            pro_eid(&self.id, "starter-chips-vtx"),
            state_key,
            inner,
        ))
    }

    fn render_turn(&self, turn: &AssistantTurn, elapsed_ms: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        match turn {
            AssistantTurn::User {
                text,
                folder,
                attachments,
                ..
            } => div()
                .w_full()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(6.))
                .when_some(folder.clone(), |d, fld| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(tray_folder_icon(13.))
                            .child(fld),
                    )
                })
                .when(!attachments.is_empty(), |d| {
                    d.child(div().flex().flex_wrap().justify_end().gap(px(6.)).children(
                        attachments.iter().map(|att| {
                            div()
                                .h(px(26.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .text_size(px(12.))
                                .text_color(p.ink)
                                .child(pro_icon(ProIcon::Paperclip, 12., p.muted))
                                .child(att.name.clone())
                        }),
                    ))
                })
                .child(
                    div()
                        .max_w(relative(0.84))
                        .px(px(14.))
                        .py(px(9.))
                        .rounded(px(16.))
                        .bg(p.chip)
                        .text_size(px(13.5))
                        .line_height(px(21.))
                        .text_color(p.ink)
                        .child(text.clone()),
                ),
            AssistantTurn::Assistant {
                id,
                trace_summary,
                trace_steps,
                text,
                artifact,
                streaming,
            } => {
                let expanded = self.expanded_traces.iter().any(|x| x == id);
                let tid = id.clone();
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .child(pro_agent_avatar(
                        "kui-assistant",
                        p.blue,
                        24.,
                        *streaming,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("trace-{tid}")))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(24.))
                                    .px(px(6.))
                                    .ml(px(-6.))
                                    .rounded(px(6.))
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .child(pro_icon(
                                        if expanded {
                                            ProIcon::ChevronDown
                                        } else {
                                            ProIcon::ChevronRight
                                        },
                                        12.,
                                        p.faint,
                                    ))
                                    .child(trace_summary.clone()),
                                cx,
                                move |this, _, cx| {
                                    if let Some(pos) =
                                        this.expanded_traces.iter().position(|x| x == &tid)
                                    {
                                        this.expanded_traces.remove(pos);
                                    } else {
                                        this.expanded_traces.push(tid.clone());
                                    }
                                    cx.notify();
                                },
                            ))
                            .child({
                                let steps_snapshot = trace_steps.clone();
                                let estimate_h = (steps_snapshot.len() as f32) * 20. + 4.;
                                pro_fold(
                                    pro_eid(&self.id, format!("trace-fold-{id}")),
                                    expanded,
                                    estimate_h,
                                    8.,
                                    move || {
                                        div()
                                            .pl(px(10.))
                                            .border_l_1()
                                            .border_color(p.line)
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.))
                                            .children(steps_snapshot.into_iter().map(|st| {
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(p.muted)
                                                    .child(st)
                                            }))
                                            .into_any_element()
                                    },
                                )
                            })
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .line_height(px(21.))
                                    .text_color(p.ink)
                                    .child(text.clone()),
                            )
                            .when_some(artifact.clone(), |d, art| {
                                let art_name = art.name.clone();
                                let glyph_c = if art.is_sheet { p.green } else { p.blue };
                                let glyph_svg = if art.is_sheet {
                                    ART_SHEET_SVG
                                } else {
                                    ART_DOC_SVG
                                };
                                d.child(
                                    div().pt(px(2.)).flex().child(activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("art-{art_name}")))
                                            .role(Role::Button)
                                            .tab_index(0)
                                            .h(px(28.))
                                            .pl(px(8.))
                                            .pr(px(10.))
                                            .rounded(px(7.))
                                            .bg(p.chip)
                                            .hover(move |s| s.bg(p.chip_hover))
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .text_size(px(12.))
                                            .text_color(p.ink)
                                            .cursor_pointer()
                                            .child(pro_svg("0 0 12 12", glyph_svg, 12., glyph_c))
                                            .child(art.name),
                                        cx,
                                        move |this, _, cx| {
                                            this.shell
                                                .update(cx, |s, cx| s.set_panel_open(true, cx));
                                            cx.emit(AssistantHomeEvent::OpenArtifact(
                                                art_name.clone(),
                                            ));
                                        },
                                    )),
                                )
                            }),
                    )
            }
        }
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editor(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        self.advance_stream(w, reduced);

        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.;
        let skel_phase = if reduced {
            0.
        } else {
            (elapsed_ms / 1000.) * 0.75
        };
        if (self.loading || self.recording || self.is_streaming()) && !reduced {
            w.request_animation_frame();
        }

        let measure_cell = self.measured_width.clone();
        let page_w = measure_cell.get();
        let narrow = page_w < 600.;
        let compact_composer = page_w < 430.;

        let body = if self.turns.is_empty() {
            let start_w = (page_w - if narrow { 24. } else { 48. }).clamp(260., 672.);
            let greeting_text: SharedString = self.greeting.clone().unwrap_or_else(|| {
                tr(
                    cx,
                    "Good morning! What's next on your list?",
                    "早上好！今天想先推进哪件事？",
                )
                .into()
            });
            div()
                .id(pro_eid(&self.id, "start-scroll"))
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .my_auto()
                        .w_full()
                        .max_w(px(672. + if narrow { 24. } else { 48. }))
                        .px(px(if narrow { 12. } else { 24. }))
                        .py(px(if narrow { 44. } else { 64. }))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(if narrow { 23. } else { 28. }))
                                .line_height(px(if narrow { 29. } else { 34. }))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .text_center()
                                .child(greeting_text),
                        )
                        .child(div().mt(px(if narrow { 22. } else { 30. })).w_full().child(
                            self.render_composer(compact_composer, elapsed_ms, skel_phase, cx),
                        ))
                        .child(self.render_starter_chips(start_w, skel_phase, cx)),
                )
        } else {
            let turns_snapshot = self.turns.clone();
            div()
                .id(pro_eid(&self.id, "turns-body"))
                .size_full()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(
                    div()
                        .h(px(44.))
                        .px(px(16.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(pro_btn(
                            pro_eid(&self.id, "back-to-start"),
                            tr(cx, "New chat", "新建对话"),
                            Some(ProIcon::NewChat),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| this.reset_conversation(cx),
                        )),
                )
                .child(
                    div()
                        .id(pro_eid(&self.id, "chat-scroll"))
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .px(px(if narrow { 16. } else { 32. }))
                        .pt(px(16.))
                        .pb(px(24.))
                        .child(
                            div()
                                .w_full()
                                .max_w(px(672.))
                                .mx_auto()
                                .flex()
                                .flex_col()
                                .gap(px(22.))
                                .children(
                                    turns_snapshot
                                        .iter()
                                        .map(|t| self.render_turn(t, elapsed_ms, cx)),
                                ),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .px(px(if narrow { 8. } else { 32. }))
                        .pb(px(if narrow { 10. } else { 24. }))
                        .flex()
                        .justify_center()
                        .child(div().w_full().max_w(px(672.)).child(self.render_composer(
                            compact_composer,
                            elapsed_ms,
                            skel_phase,
                            cx,
                        ))),
                )
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key))
            .relative()
            .size_full()
            .bg(p.panel)
            .flex()
            .flex_col()
            .overflow_hidden()
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
            .child(pro_view_transition_fill(
                pro_eid(&self.id, "home-mode-vtx"),
                if self.turns.is_empty() {
                    "start"
                } else {
                    "chat"
                },
                body,
            ))
    }
}

impl Render for AssistantHomeScreen {
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
        AssistantApprovalMode, AssistantHomeScreen, StarterChipKind, default_models,
        default_starter_chips, synthesize_reply,
    };

    #[test]
    fn default_catalog_matches_spec_chips_and_models() {
        let chips = default_starter_chips();
        assert_eq!(chips.len(), 7);
        assert_eq!(chips[0].label.as_ref(), "Company knowledge");
        assert_eq!(chips[0].kind, StarterChipKind::CompanyKnowledge);
        assert_eq!(chips[6].label.as_ref(), "Visualize data");
        assert_eq!(chips[6].kind, StarterChipKind::VisualizeData);

        let models = default_models();
        assert_eq!(models[0].name.as_ref(), "Auto");
        assert!(models[0].provider.is_none());
        assert_eq!(
            AssistantApprovalMode::default(),
            AssistantApprovalMode::RequestApproval
        );
    }

    #[test]
    fn synthesize_reply_routes_budget_and_document_artifacts() {
        let (_, steps, _, sheet_art) = synthesize_reply("Build a Q3 budget table by channel");
        assert!(!steps.is_empty());
        let art = sheet_art.expect("budget prompt produces a sheet artifact");
        assert!(art.is_sheet);
        assert_eq!(art.name.as_ref(), "budget-by-channel.csv");

        let (_, _, _, doc_art) = synthesize_reply("Draft Q3 launch messaging");
        let doc = doc_art.expect("launch prompt produces a document artifact");
        assert!(!doc.is_sheet);
        assert_eq!(doc.name.as_ref(), "q3-launch-plan.md");
        let _ = std::mem::size_of::<AssistantHomeScreen>();
    }
}
