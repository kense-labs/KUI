//! AI workspace overview with composer, 40-week activity heatmap, chats, usage, and files.
//!
//! Renders [`AssistantOverviewScreen`] inside [`AppShell`] (`AppShellPreset::Chat`, active `"New chat"`),
//! pairing the Kense KUI brand header and tool-enabled prompt composer with a responsive four-column
//! overview board: daily activity heatmap with keyboard navigation, recent chats, credit usage meter,
//! generated files with extension-preserving truncation, and month-over-month deltas.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
            pro_presence, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_switch,
            pro_view_transition, pro_view_transition_fill, tr,
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

const CHAT_DOTS_SVG: &str = r#"<path d="M21.5 12C21.5 17.2467 17.2467 21.5 12 21.5C10.3719 21.5 8.8394 21.0904 7.5 20.3687C5.63177 19.362 4.37462 20.2979 3.26592 20.4658C3.09774 20.4913 2.93024 20.4302 2.80997 20.31C2.62741 20.1274 2.59266 19.8451 2.6935 19.6074C3.12865 18.5818 3.5282 16.6382 2.98341 15C2.6698 14.057 2.5 13.0483 2.5 12C2.5 6.75329 6.75329 2.5 12 2.5C17.2467 2.5 21.5 6.75329 21.5 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12.1257 12H12.0007M8.125 12H8M16.125 12H16M12.2507 12C12.2507 12.1381 12.1388 12.25 12.0007 12.25C11.8627 12.25 11.7507 12.1381 11.7507 12C11.7507 11.8619 11.8627 11.75 12.0007 11.75C12.1388 11.75 12.2507 11.8619 12.2507 12ZM8.25 12C8.25 12.1381 8.13807 12.25 8 12.25C7.86193 12.25 7.75 12.1381 7.75 12C7.75 11.8619 7.86193 11.75 8 11.75C8.13807 11.75 8.25 11.8619 8.25 12ZM16.25 12C16.25 12.1381 16.1381 12.25 16 12.25C15.8619 12.25 15.75 12.1381 15.75 12C15.75 11.8619 15.8619 11.75 16 11.75C16.1381 11.75 16.25 11.8619 16.25 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const USAGE_TREND_SVG: &str = r#"<path d="M4.31802 19.682C3 18.364 3 16.2426 3 12C3 7.75736 3 5.63604 4.31802 4.31802C5.63604 3 7.75736 3 12 3C16.2426 3 18.364 3 19.682 4.31802C21 5.63604 21 7.75736 21 12C21 16.2426 21 18.364 19.682 19.682C18.364 21 16.2426 21 12 21C7.75736 21 5.63604 21 4.31802 19.682Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7 14L9.79289 11.2071C10.1834 10.8166 10.8166 10.8166 11.2071 11.2071L12.7929 12.7929C13.1834 13.1834 13.8166 13.1834 14.2071 12.7929L17 10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const FILES_HEAD_SVG: &str = r#"<path d="M8 7L16 7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M8 11L12 11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M13 21.5V21C13 18.1716 13 16.7574 13.8787 15.8787C14.7574 15 16.1716 15 19 15H19.5M20 13.3431V10C20 6.22876 20 4.34315 18.8284 3.17157C17.6569 2 15.7712 2 12 2C8.22877 2 6.34315 2 5.17157 3.17157C4 4.34314 4 6.22876 4 10L4 14.5442C4 17.7892 4 19.4117 4.88607 20.5107C5.06508 20.7327 5.26731 20.9349 5.48933 21.1139C6.58831 22 8.21082 22 11.4558 22C12.1614 22 12.5141 22 12.8372 21.886C12.9044 21.8623 12.9702 21.835 13.0345 21.8043C13.3436 21.6564 13.593 21.407 14.0919 20.9081L18.8284 16.1716C19.4065 15.5935 19.6955 15.3045 19.8478 14.9369C20 14.5694 20 14.1606 20 13.3431Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const FILE_DOC_GLYPH_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

const FILE_SHEET_GLYPH_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;

/// Exact 40-week x 7-day activity intensity matrix (`0..=4`) from the reference overview board.
const DEFAULT_HEATMAP_LEVELS: [[u8; 7]; 40] = [
    [0, 2, 0, 1, 1, 0, 0],
    [0, 1, 1, 0, 0, 0, 0],
    [0, 1, 2, 1, 1, 0, 0],
    [1, 2, 1, 2, 2, 0, 0],
    [2, 1, 2, 2, 1, 0, 0],
    [1, 2, 2, 1, 1, 0, 0],
    [1, 2, 1, 2, 1, 0, 0],
    [1, 1, 1, 1, 1, 0, 0],
    [2, 2, 2, 2, 1, 0, 0],
    [1, 1, 1, 2, 2, 0, 0],
    [2, 2, 2, 3, 2, 0, 0],
    [2, 1, 1, 2, 2, 1, 0],
    [3, 2, 2, 2, 2, 0, 1],
    [1, 2, 2, 2, 2, 1, 1],
    [3, 2, 2, 1, 3, 1, 0],
    [3, 2, 2, 2, 3, 0, 0],
    [3, 3, 2, 2, 2, 0, 0],
    [3, 2, 3, 3, 1, 0, 1],
    [2, 3, 1, 3, 2, 0, 0],
    [3, 2, 3, 1, 3, 1, 0],
    [2, 2, 2, 2, 3, 0, 0],
    [1, 2, 2, 3, 1, 0, 1],
    [2, 3, 1, 2, 3, 1, 0],
    [2, 3, 2, 2, 2, 1, 1],
    [3, 3, 2, 2, 2, 0, 1],
    [2, 3, 3, 2, 2, 2, 1],
    [3, 3, 2, 2, 2, 0, 1],
    [3, 2, 3, 3, 2, 2, 1],
    [2, 3, 2, 3, 3, 2, 1],
    [3, 2, 3, 3, 2, 2, 1],
    [2, 2, 4, 3, 3, 1, 0],
    [4, 2, 3, 3, 3, 2, 2],
    [3, 3, 2, 3, 4, 1, 2],
    [3, 3, 3, 3, 3, 2, 1],
    [3, 3, 3, 4, 3, 1, 1],
    [3, 3, 2, 3, 3, 1, 2],
    [3, 2, 4, 4, 4, 1, 1],
    [3, 3, 4, 3, 2, 1, 2],
    [4, 4, 4, 4, 3, 1, 2],
    [3, 3, 3, 4, 3, 2, 2],
];

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
    const MARKUP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 49.7096 59.6884" width="48" height="48" fill="none"><path fill-rule="evenodd" clip-rule="evenodd" d="M0 45.9002L9.1618 38.8247C14.0602 45.1745 19.2307 48.1679 24.9455 48.1679C30.6603 48.1679 35.7401 45.2652 40.3663 38.9154L49.7096 45.8094C42.997 54.8805 34.6516 59.6882 24.9455 59.6882C15.3301 59.6882 6.89403 54.8805 0 45.9002Z" fill="url(#cuov0)"></path><path fill-rule="evenodd" clip-rule="evenodd" d="M24.9535 15.3301L8.62558 29.3903L1.09657 20.5914L25.0442 0L48.8105 20.5914L41.1908 29.2996L24.9535 15.3301Z" fill="url(#cuov1)"></path><defs><linearGradient id="cuov0" x1="0" y1="49.2666" x2="49.75" y2="49.2666" gradientUnits="userSpaceOnUse"><stop stop-color="#8930FD"></stop><stop offset="1" stop-color="#49CCF9"></stop></linearGradient><linearGradient id="cuov1" x1="1.09231" y1="14.6901" x2="48.7603" y2="14.6901" gradientUnits="userSpaceOnUse"><stop stop-color="#FF02F0"></stop><stop offset="1" stop-color="#FFC800"></stop></linearGradient></defs></svg>"##;
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

fn format_thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + 3);
    for (idx, ch) in s.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Split a filename into `(stem, extension_with_dot)` so long stems truncate without hiding `.md` or `.csv`.
pub fn split_filename_extension(filename: &str) -> (&str, &str) {
    if let Some(dot_idx) = filename.rfind('.')
        && dot_idx > 0
        && dot_idx + 1 < filename.len()
    {
        (&filename[..dot_idx], &filename[dot_idx..])
    } else {
        (filename, "")
    }
}

fn mix_hsla(fg: Hsla, bg: Hsla, weight: f32) -> Hsla {
    let a: Rgba = fg.into();
    let b: Rgba = bg.into();
    let w = weight.clamp(0., 1.);
    Rgba {
        r: b.r + (a.r - b.r) * w,
        g: b.g + (a.g - b.g) * w,
        b: b.b + (a.b - b.b) * w,
        a: 1.0,
    }
    .into()
}

/// Action approval policy chosen in the overview composer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverviewApprovalMode {
    #[default]
    RequestApproval,
    AutoApproveSafe,
    FullAccess,
}

impl OverviewApprovalMode {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "请求审批"),
            Self::AutoApproveSafe => tr(cx, "Auto-approve safe", "只读自动批准"),
            Self::FullAccess => tr(cx, "Full access", "完全自动执行"),
        }
    }
}

/// Model option offered in the overview composer model selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverviewModelOption {
    pub id: SharedString,
    pub name: SharedString,
    pub provider: Option<SharedString>,
    pub summary: SharedString,
}

impl OverviewModelOption {
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

/// One day cell inside the 40-week (`280`-day) activity heatmap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverviewActivityDay {
    pub index: usize,
    pub date_label: SharedString,
    pub level: u8,
    pub chats: u32,
}

/// One recent conversation row in the `Recent chats` card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverviewChatItem {
    pub id: SharedString,
    pub title: SharedString,
    pub updated_label: SharedString,
}

impl OverviewChatItem {
    /// Construct a recent chat row.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        updated_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            updated_label: updated_label.into(),
        }
    }
}

/// File glyph category in the `Files made for you` card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverviewFileKind {
    #[default]
    Document,
    Spreadsheet,
    Code,
}

/// One generated workspace file row in the `Files made for you` card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverviewFileItem {
    pub id: SharedString,
    pub filename: SharedString,
    pub updated_label: SharedString,
    pub kind: OverviewFileKind,
}

impl OverviewFileItem {
    /// Construct a generated file row.
    pub fn new(
        id: impl Into<SharedString>,
        filename: impl Into<SharedString>,
        updated_label: impl Into<SharedString>,
        kind: OverviewFileKind,
    ) -> Self {
        Self {
            id: id.into(),
            filename: filename.into(),
            updated_label: updated_label.into(),
            kind,
        }
    }
}

/// Credit usage meter values shown in the `Usage` card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverviewUsageStats {
    pub used_credits: u32,
    pub limit_credits: u32,
    pub resets_label: SharedString,
}

impl Default for OverviewUsageStats {
    fn default() -> Self {
        Self {
            used_credits: 1_284,
            limit_credits: 2_000,
            resets_label: "Resets Dec 1".into(),
        }
    }
}

impl OverviewUsageStats {
    /// Normalized fill ratio in `0.0..=1.0`.
    pub fn share(&self) -> f32 {
        if self.limit_credits == 0 {
            0.0
        } else {
            (self.used_credits as f32 / self.limit_credits as f32).clamp(0.0, 1.0)
        }
    }

    /// Return `true` when `used_credits` exceeds `limit_credits`.
    pub fn is_over_limit(&self) -> bool {
        self.limit_credits > 0 && self.used_credits > self.limit_credits
    }
}

/// Month-over-month comparison metrics shown in the `This month` card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverviewMonthlyStats {
    pub chats_count: u32,
    pub chats_delta: i32,
    pub files_count: u32,
    pub files_delta: i32,
}

impl Default for OverviewMonthlyStats {
    fn default() -> Self {
        Self {
            chats_count: 36,
            chats_delta: 12,
            files_count: 14,
            files_delta: -3,
        }
    }
}

/// Events emitted by [`AssistantOverviewScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssistantOverviewEvent {
    Navigate(SharedString),
    SubmitPrompt {
        prompt: SharedString,
        model_id: SharedString,
        approval: OverviewApprovalMode,
        folder: Option<SharedString>,
    },
    OpenChat(SharedString),
    OpenFile(SharedString),
    InspectDay {
        date_label: SharedString,
        chats: u32,
    },
    SelectModel(SharedString),
    SelectFolder(Option<SharedString>),
    ToggleIntegration {
        app: SharedString,
        enabled: bool,
    },
}

impl EventEmitter<AssistantOverviewEvent> for AssistantOverviewScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenMenu {
    Add,
    Approval,
    Model,
    Folder,
    Apps,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActiveTurn {
    User {
        text: SharedString,
        folder: Option<SharedString>,
    },
    Assistant {
        text: SharedString,
        artifact: SharedString,
        is_sheet: bool,
        streaming: bool,
    },
}

fn default_activity_days() -> Vec<OverviewActivityDay> {
    const MONTHS: [(&str, u32); 10] = [
        ("Feb", 28),
        ("Mar", 31),
        ("Apr", 30),
        ("May", 31),
        ("Jun", 30),
        ("Jul", 31),
        ("Aug", 31),
        ("Sep", 30),
        ("Oct", 31),
        ("Nov", 30),
    ];
    let mut month_idx = 0usize;
    let mut day_of_month = 9u32;
    let mut bonus_remaining = 20u32;
    let mut days = Vec::with_capacity(280);

    for (wi, week) in DEFAULT_HEATMAP_LEVELS.iter().enumerate() {
        for (di, &lvl) in week.iter().enumerate() {
            let index = wi * 7 + di;
            let (m_name, m_len) = MONTHS[month_idx.min(MONTHS.len() - 1)];
            let date_label: SharedString = format!("{m_name} {day_of_month}").into();

            day_of_month += 1;
            if day_of_month > m_len && month_idx + 1 < MONTHS.len() {
                month_idx += 1;
                day_of_month = 1;
            }

            let mut chats = match lvl {
                0 => 0,
                1 => 1,
                2 => 3,
                3 => 6,
                _ => 9,
            };
            if lvl >= 3 && bonus_remaining > 0 {
                chats += 1;
                bonus_remaining -= 1;
            }
            days.push(OverviewActivityDay {
                index,
                date_label,
                level: lvl,
                chats,
            });
        }
    }
    days
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

fn default_models() -> Vec<OverviewModelOption> {
    vec![
        OverviewModelOption::new("auto", "Auto", None, "Best model for each task"),
        OverviewModelOption::new(
            "claude-sonnet-4-5",
            "Claude Sonnet 4.5",
            Some("Claude"),
            "Everyday writing & code",
        ),
        OverviewModelOption::new(
            "claude-opus-4-1",
            "Claude Opus 4.1",
            Some("Claude"),
            "Deep reasoning & synthesis",
        ),
        OverviewModelOption::new(
            "gpt-5",
            "GPT-5",
            Some("OpenAI"),
            "Fast multimodal reasoning",
        ),
        OverviewModelOption::new("o3", "o3", Some("OpenAI"), "Step-by-step analysis"),
        OverviewModelOption::new(
            "gemini-2-5-pro",
            "Gemini 2.5 Pro",
            Some("Gemini"),
            "Long context & workspace docs",
        ),
        OverviewModelOption::new(
            "deepseek-r1",
            "DeepSeek R1",
            Some("DeepSeek"),
            "Structured verification",
        ),
    ]
}

fn step_heatmap_index(current: usize, total: usize, key: &str) -> Option<usize> {
    if total == 0 {
        return None;
    }
    let last = total - 1;
    match key {
        "left" | "arrowleft" => Some(current.saturating_sub(7)),
        "right" | "arrowright" => Some((current + 7).min(last)),
        "up" | "arrowup" => Some(current.saturating_sub(1)),
        "down" | "arrowdown" => Some((current + 1).min(last)),
        "home" => Some(0),
        "end" => Some(last),
        _ => None,
    }
}

/// AI workspace overview screen (`assistant-overview`) with prompt composer and bento metrics board.
pub struct AssistantOverviewScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    brand_title: SharedString,
    loading: bool,
    empty: bool,
    composer_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    open_menu: Option<OpenMenu>,
    last_menu: Cell<OpenMenu>,
    approval_mode: OverviewApprovalMode,
    models: Vec<OverviewModelOption>,
    selected_model: usize,
    folders: Vec<SharedString>,
    active_folder: Option<SharedString>,
    connected_apps: Vec<(SharedString, bool)>,
    web_search_enabled: bool,
    attachments: Vec<SharedString>,
    activity_days: Vec<OverviewActivityDay>,
    active_day: Option<usize>,
    last_active_day: Cell<usize>,
    total_files_count: u32,
    total_automations_count: u32,
    date_range_label: SharedString,
    recent_chats: Vec<OverviewChatItem>,
    more_chats_count: u32,
    usage: OverviewUsageStats,
    files: Vec<OverviewFileItem>,
    more_files_count: u32,
    monthly: OverviewMonthlyStats,
    turns: Vec<ActiveTurn>,
    recording: bool,
    stream_state: Option<(SharedString, Instant)>,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AssistantOverviewScreen {
    /// Construct a new [`AssistantOverviewScreen`] hosted in [`AppShell`] (`Chat` preset, `"Acme"`).
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
                cx.emit(AssistantOverviewEvent::Navigate(label.clone()));
            }
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            focus,
            shell,
            standalone: false,
            brand_title: "KUI".into(),
            loading: false,
            empty: false,
            composer_editor: None,
            editor_focused: false,
            open_menu: None,
            last_menu: Cell::new(OpenMenu::Model),
            approval_mode: OverviewApprovalMode::RequestApproval,
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
            attachments: Vec::new(),
            activity_days: default_activity_days(),
            active_day: None,
            last_active_day: Cell::new(0),
            total_files_count: 214,
            total_automations_count: 38,
            date_range_label: "Feb – Nov".into(),
            recent_chats: vec![
                OverviewChatItem::new("q3-launch", "Q3 launch messaging", "2 hours ago"),
                OverviewChatItem::new(
                    "linen-hooks",
                    "Ad hooks for the linen overshirt",
                    "Yesterday",
                ),
                OverviewChatItem::new("partner-logos", "Logo usage on partner sites", "3 days ago"),
            ],
            more_chats_count: 6,
            usage: OverviewUsageStats::default(),
            files: vec![
                OverviewFileItem::new(
                    "plan-md",
                    "q3-launch-plan.md",
                    "1 hour ago",
                    OverviewFileKind::Document,
                ),
                OverviewFileItem::new(
                    "budget-csv",
                    "budget-by-channel.csv",
                    "Yesterday",
                    OverviewFileKind::Spreadsheet,
                ),
                OverviewFileItem::new(
                    "pricing-csv",
                    "competitor-pricing.csv",
                    "Tue",
                    OverviewFileKind::Spreadsheet,
                ),
            ],
            more_files_count: 11,
            monthly: OverviewMonthlyStats::default(),
            turns: Vec::new(),
            recording: false,
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

    /// Override the product wordmark shown next to the Kense mark above the composer.
    pub fn with_brand_title(mut self, title: impl Into<SharedString>) -> Self {
        self.brand_title = title.into();
        self
    }

    /// Override the recent chat items shown in the `Recent chats` card.
    pub fn with_recent_chats(mut self, chats: Vec<OverviewChatItem>, more_count: u32) -> Self {
        self.recent_chats = chats;
        self.more_chats_count = more_count;
        self
    }

    /// Override the credit usage metrics shown in the `Usage` card.
    pub fn with_usage(mut self, usage: OverviewUsageStats) -> Self {
        self.usage = usage;
        self
    }

    /// Override the generated workspace files shown in `Files made for you`.
    pub fn with_files(mut self, files: Vec<OverviewFileItem>, more_count: u32) -> Self {
        self.files = files;
        self.more_files_count = more_count;
        self
    }

    /// Override the month-over-month stats shown in `This month`.
    pub fn with_monthly_stats(mut self, stats: OverviewMonthlyStats) -> Self {
        self.monthly = stats;
        self
    }

    /// Start in skeleton loading mode while workspace metrics load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Start in empty workspace mode (`"Nothing here yet"` under the composer).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle the skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle the empty workspace state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the total chat count summed across all 40 weeks in the activity heatmap.
    pub fn total_activity_chats(&self) -> u32 {
        self.activity_days.iter().map(|d| d.chats).sum()
    }

    /// Reset an active conversation back to the overview dashboard.
    pub fn reset_conversation(&mut self, cx: &mut Context<Self>) {
        self.turns.clear();
        self.stream_state = None;
        self.open_menu = None;
        cx.notify();
    }

    /// Submit a prompt from the composer, transitioning into an active reply view.
    pub fn submit_prompt(&mut self, raw: impl Into<SharedString>, cx: &mut Context<Self>) -> bool {
        if self.loading {
            return false;
        }
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() && self.attachments.is_empty() {
            return false;
        }
        let prompt_text: SharedString = if trimmed.is_empty() {
            "Summarize attached workspace files.".into()
        } else {
            trimmed.into()
        };
        self.attachments.clear();
        let model_id = self
            .models
            .get(self.selected_model)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "auto".into());
        let folder = self.active_folder.clone();
        let is_sheet = prompt_text.to_ascii_lowercase().contains("budget")
            || prompt_text.to_ascii_lowercase().contains("csv");
        let artifact: SharedString = if is_sheet {
            "budget-by-channel.csv".into()
        } else {
            "q3-launch-plan.md".into()
        };
        let full_reply: SharedString = "Pulled the latest numbers across Q3 Campaign and updated your workspace summary with current channel allocations and launch owners.".into();

        let reduced = Theme::global(cx).reduced_motion;
        let initial_text: SharedString = if reduced {
            full_reply.clone()
        } else {
            full_reply.chars().take(14).collect::<String>().into()
        };

        self.turns.push(ActiveTurn::User {
            text: prompt_text.clone(),
            folder: folder.clone(),
        });
        self.turns.push(ActiveTurn::Assistant {
            text: initial_text,
            artifact,
            is_sheet,
            streaming: !reduced,
        });
        self.empty = false;
        self.open_menu = None;
        if !reduced {
            self.stream_state = Some((full_reply, Instant::now()));
        }

        cx.emit(AssistantOverviewEvent::SubmitPrompt {
            prompt: prompt_text,
            model_id,
            approval: self.approval_mode,
            folder,
        });
        cx.notify();
        true
    }

    fn stop_streaming(&mut self, cx: &mut Context<Self>) {
        if self.stream_state.take().is_some() {
            if let Some(ActiveTurn::Assistant { streaming, .. }) = self.turns.last_mut() {
                *streaming = false;
            }
            cx.notify();
        }
    }

    fn is_streaming(&self) -> bool {
        self.stream_state.is_some()
    }

    fn advance_stream(&mut self, w: &mut Window, reduced: bool) {
        let Some((full_text, started_at)) = self.stream_state.clone() else {
            return;
        };
        let elapsed_ms = started_at.elapsed().as_secs_f32() * 1000.;
        let ratio = if reduced {
            1.0
        } else {
            (elapsed_ms / 660.).clamp(0.08, 1.0)
        };
        let chars: Vec<char> = full_text.chars().collect();
        let take_n = ((chars.len() as f32 * ratio).ceil() as usize).min(chars.len());
        let prefix: String = chars.into_iter().take(take_n).collect();
        let done_now = ratio >= 0.999;

        if let Some(ActiveTurn::Assistant {
            text, streaming, ..
        }) = self.turns.last_mut()
        {
            *text = prefix.into();
            if done_now {
                *streaming = false;
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
        let k = ev.keystroke.key.to_ascii_lowercase();
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
            if self.open_menu.take().is_some() || self.active_day.take().is_some() {
                cx.notify();
            } else if self.recording {
                self.recording = false;
                cx.notify();
            } else if self.is_streaming() {
                self.stop_streaming(cx);
            }
            return;
        }
        if self.turns.is_empty() && !self.loading && !self.empty {
            let total = self.activity_days.len();
            let base = self.active_day.unwrap_or_else(|| total.saturating_sub(1));
            if let Some(next_idx) = step_heatmap_index(base, total, &k) {
                self.active_day = Some(next_idx);
                if let Some(day) = self.activity_days.get(next_idx) {
                    cx.emit(AssistantOverviewEvent::InspectDay {
                        date_label: day.date_label.clone(),
                        chats: day.chats,
                    });
                }
                cx.notify();
            }
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
                .id(pro_eid(&self.id, "ov-menu-add"))
                .role(Role::Menu)
                .w(px(252.))
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
                        .id(pro_eid(&self.id, "ov-add-file"))
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
                        this.attachments.push("q3-launch-plan.md".into());
                        this.open_menu = None;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "ov-add-web"))
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
                        let label_owned: SharedString = label.into();
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("ov-plugin-{pi}")))
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
                                this.attachments.push(label_owned.clone());
                                this.open_menu = None;
                                cx.notify();
                            },
                        )
                    }),
                ),
            OpenMenu::Approval => {
                let appr_sensor = pro_selection_sensor();
                div()
                    .id(pro_eid(&self.id, "ov-menu-approval"))
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
                                pro_eid(&self.id, "ov-appr-pill"),
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
                                        OverviewApprovalMode::RequestApproval,
                                        tr(cx, "Request approval", "请求审批"),
                                        tr(
                                            cx,
                                            "Ask before writes or external actions",
                                            "写入或外部操作前询问",
                                        ),
                                        false,
                                    ),
                                    (
                                        OverviewApprovalMode::AutoApproveSafe,
                                        tr(cx, "Auto-approve safe", "只读自动批准"),
                                        tr(
                                            cx,
                                            "Read workspace docs without pausing",
                                            "安全读取文档无需暂停",
                                        ),
                                        false,
                                    ),
                                    (
                                        OverviewApprovalMode::FullAccess,
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
                                        } else if mode == OverviewApprovalMode::RequestApproval {
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
                                                .id(pro_eid(&self.id, format!("ov-appr-{ai}")))
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
                    .id(pro_eid(&self.id, "ov-menu-model"))
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
                        pro_eid(&self.id, "ov-mdl-pill"),
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
                                .id(pro_eid(&self.id, format!("ov-mdl-{mi}")))
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
                                cx.emit(AssistantOverviewEvent::SelectModel(mid.clone()));
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
                            pro_eid(&self.id, "ov-fld-pill"),
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
                            .id(pro_eid(&self.id, format!("ov-fld-{fi}")))
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
                            cx.emit(AssistantOverviewEvent::SelectFolder(next));
                            cx.notify();
                        },
                    ));
                }
                div()
                    .id(pro_eid(&self.id, "ov-menu-folder"))
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
                    .id(pro_eid(&self.id, "ov-menu-apps"))
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
                                pro_eid(&self.id, format!("ov-app-sw-{ai}")),
                                app_name.clone(),
                                *enabled,
                                cx,
                                move |this, _, cx| {
                                    if let Some((_, flag)) =
                                        this.connected_apps.iter_mut().find(|(n, _)| n == &app_str)
                                    {
                                        *flag = next_on;
                                        cx.emit(AssistantOverviewEvent::ToggleIntegration {
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
                            .id(pro_eid(&self.id, "ov-apps-manage"))
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
                            cx.emit(AssistantOverviewEvent::Navigate("Integrations".into()));
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
        let is_alert = self.approval_mode == OverviewApprovalMode::FullAccess;
        let appr_fg = if is_alert { p.red_ink } else { p.muted };

        let cur_model = self
            .models
            .get(self.selected_model)
            .cloned()
            .unwrap_or_else(|| OverviewModelOption::new("auto", "Auto", None, ""));

        let right_bar = if self.recording {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .h(px(20.))
                        .mr(px(4.))
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        .children((0..9usize).map(|bi| {
                            let h = if reduced {
                                8.
                            } else {
                                4. + 12. * ((elapsed_ms / 170. + bi as f32 * 0.65).sin().abs())
                            };
                            div().w(px(2.)).h(px(h)).rounded(px(1.)).bg(p.blue)
                        })),
                )
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "ov-rec-cancel"))
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
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "ov-rec-commit"))
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
                        if let Some(ed) = this.composer_editor.clone() {
                            ed.update(cx, |e, cx| {
                                e.set_value(
                                    "Compare this month's Q3 campaign spend against budget",
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
                        .id(pro_eid(&self.id, "ov-composer-model"))
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
                    pro_eid(&self.id, "ov-composer-mic"),
                    tr(cx, "Voice dictation", "语音输入"),
                    ProIcon::Mic,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.recording = true;
                        cx.notify();
                    },
                ))
                .child(if streaming {
                    activate(
                        div()
                            .id(pro_eid(&self.id, "ov-composer-stop"))
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
                            .id(pro_eid(&self.id, "ov-composer-send"))
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
            .children(self.attachments.iter().enumerate().map(|(ai, name)| {
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
                    .child(name.clone())
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("ov-rm-att-{ai}")),
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
                    "Listening… Compare this month's Q3 campaign spend against budget…",
                    "正在聆听… 对比本月 Q3 活动支出与预算…",
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
                pro_eid(&self.id, "ov-composer-att-fold"),
                !self.attachments.is_empty(),
                36.,
                0.,
                move || att_strip.into_any_element(),
            ))
            .child(pro_view_transition(
                pro_eid(&self.id, "ov-composer-input-vtx"),
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
                                pro_eid(&self.id, "ov-composer-add"),
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
                                    .id(pro_eid(&self.id, "ov-composer-approval"))
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
                    .id(pro_eid(&self.id, "ov-tray-apps"))
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
                    .id(pro_eid(&self.id, "ov-tray-folder"))
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
        let pop_vtx_id = pro_eid(&self.id, "ov-pop-chat-vtx");
        let pop_el = self.render_popover_menu(menu_kind, cx);

        let mut wrap = div().relative().w_full().flex().flex_col().gap(px(6.));
        if in_chat {
            wrap = wrap.child(pro_presence(
                pro_eid(&self.id, "ov-pop-chat-pres"),
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
            let pop_vtx_start_id = pro_eid(&self.id, "ov-pop-start-vtx");
            let menu_key_start: SharedString = format!("{menu_kind:?}").into();
            let pop_el_start = self.render_popover_menu(menu_kind, cx);
            wrap = wrap.child(pro_presence(
                pro_eid(&self.id, "ov-pop-start-pres"),
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

    fn render_activity_card(&self, card_inner_w: f32, skel_phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let ov_blue: Hsla = rgb(0x0190fe).into();
        let l1 = mix_hsla(ov_blue, p.panel, 0.22);
        let l2 = mix_hsla(ov_blue, p.panel, 0.45);
        let l3 = mix_hsla(ov_blue, p.panel, 0.70);
        let l4 = ov_blue;

        let cell_side = ((card_inner_w - 39. * 2.) / 40.).clamp(3.8, 8.5);
        let total_chats = self.total_activity_chats();

        let mut card = div()
            .relative()
            .min_w_0()
            .px(px(12.))
            .py(px(10.))
            .rounded(px(12.))
            .bg(p.panel)
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
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(pro_icon(ProIcon::Calendar, 14., p.muted))
                            .child(tr(cx, "Activity", "活跃度")),
                    )
                    .when(!self.loading, |d| {
                        if let Some(sel_idx) = self.active_day {
                            self.last_active_day.set(sel_idx);
                        }
                        let disp_idx = self
                            .active_day
                            .unwrap_or_else(|| self.last_active_day.get());
                        let tip_text: SharedString =
                            if let Some(day) = self.activity_days.get(disp_idx) {
                                if day.chats == 0 {
                                    format!("{} · {}", day.date_label, tr(cx, "No chats", "无对话"))
                                        .into()
                                } else if day.chats == 1 {
                                    format!("{} · 1 {}", day.date_label, tr(cx, "chat", "次对话"))
                                        .into()
                                } else {
                                    format!(
                                        "{} · {} {}",
                                        day.date_label,
                                        day.chats,
                                        tr(cx, "chats", "次对话")
                                    )
                                    .into()
                                }
                            } else {
                                SharedString::default()
                            };
                        let has_day = self.active_day.is_some();
                        let tip_key: SharedString = format!("day-{disp_idx}").into();
                        let tip_vtx_id = pro_eid(&self.id, "ov-day-tip-vtx");
                        d.child(pro_presence(
                            pro_eid(&self.id, "ov-day-tip-pres"),
                            has_day,
                            0.18,
                            0.12,
                            move |a| {
                                div()
                                    .opacity(a)
                                    .child(pro_view_transition(
                                        tip_vtx_id,
                                        tip_key,
                                        div()
                                            .px(px(7.))
                                            .py(px(2.))
                                            .rounded(px(6.))
                                            .bg(p.ink)
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.panel)
                                            .child(tip_text),
                                    ))
                                    .into_any_element()
                            },
                        ))
                    }),
            );

        if self.loading {
            card = card
                .child(pro_skeleton_bar(
                    card_inner_w.max(180.),
                    cell_side * 7. + 12.,
                    6.,
                    skel_phase,
                    cx,
                ))
                .child(
                    div()
                        .mt_auto()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(168., 11., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(48., 11., 4., skel_phase, cx)),
                );
            return card;
        }

        let heat_sensor = pro_selection_sensor();
        let heat_sel_key: SharedString = self
            .active_day
            .map(|idx| format!("hday-{idx}").into())
            .unwrap_or_else(|| "none".into());
        let mut heat_grid = div()
            .id(pro_eid(&self.id, "activity-heatmap"))
            .role(Role::Region)
            .aria_label(format!(
                "{total_chats} chats over the last 40 weeks – arrow keys navigate days"
            ))
            .tab_index(0)
            .relative()
            .w_full()
            .rounded(px(4.))
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "heat-sel-pill"),
                heat_sel_key,
                heat_sensor.clone(),
                2.,
                transparent_black(),
                Some(p.ink),
                false,
            ));

        for week_idx in 0..40usize {
            let mut week_col = div().flex_1().min_w_0().flex().flex_col().gap(px(2.));
            for day_row in 0..7usize {
                let flat_idx = week_idx * 7 + day_row;
                let day = self.activity_days.get(flat_idx);
                let lvl = day.map(|d| d.level).unwrap_or(0);
                let fill = match lvl {
                    0 => p.chip,
                    1 => l1,
                    2 => l2,
                    3 => l3,
                    _ => l4,
                };
                let is_on = self.active_day == Some(flat_idx);
                let d_label = day
                    .map(|d| d.date_label.clone())
                    .unwrap_or_else(|| "Day".into());
                let d_chats = day.map(|d| d.chats).unwrap_or(0);

                week_col = week_col.child(
                    div()
                        .id(pro_eid(&self.id, format!("hcell-{flat_idx}")))
                        .relative()
                        .w_full()
                        .h(px(cell_side))
                        .rounded(px(2.))
                        .bg(fill)
                        .when(is_on, |d| d.child(pro_mark_selected(&heat_sensor)))
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.active_day = Some(flat_idx);
                                cx.emit(AssistantOverviewEvent::InspectDay {
                                    date_label: d_label.clone(),
                                    chats: d_chats,
                                });
                                cx.notify();
                            }),
                        ),
                );
            }
            heat_grid = heat_grid.child(week_col);
        }

        let summary_left = format!(
            "{} {} · {} {} · {} {}",
            format_thousands(total_chats),
            tr(cx, "chats", "次对话"),
            format_thousands(self.total_files_count),
            tr(cx, "files", "个文件"),
            self.total_automations_count,
            tr(cx, "automations", "项自动化"),
        );

        card.child(heat_grid).child(
            div()
                .mt_auto()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .text_size(px(11.5))
                .text_color(p.text)
                .child(div().min_w_0().truncate().child(summary_left))
                .child(
                    div()
                        .flex_none()
                        .text_color(p.muted)
                        .child(self.date_range_label.clone()),
                ),
        )
    }

    fn render_recent_chats_card(&self, skel_phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let card = div()
            .min_w_0()
            .px(px(12.))
            .py(px(10.))
            .rounded(px(12.))
            .bg(p.panel)
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
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(pro_icon(ProIcon::Clock, 14., p.muted))
                            .child(tr(cx, "Recent chats", "最近对话")),
                    )
                    .child(if self.loading {
                        pro_skeleton_bar(38., 12., 5., skel_phase, cx).into_any_element()
                    } else {
                        activate(
                            div()
                                .id(pro_eid(&self.id, "recent-more"))
                                .role(Role::Button)
                                .tab_index(0)
                                .mr(px(-6.))
                                .px(px(6.))
                                .py(px(2.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                .text_size(px(11.5))
                                .text_color(p.text)
                                .cursor_pointer()
                                .child(format!(
                                    "{} {}",
                                    self.more_chats_count,
                                    tr(cx, "more", "更多")
                                )),
                            cx,
                            |_, _, cx| {
                                cx.emit(AssistantOverviewEvent::Navigate("Chat".into()));
                            },
                        )
                        .into_any_element()
                    }),
            );

        if self.loading {
            let mut skel_list = div().flex().flex_col().gap(px(6.));
            for w_bar in [148., 176., 136.] {
                skel_list = skel_list.child(
                    div()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(18., 18., 6., skel_phase, cx))
                        .child(pro_skeleton_bar(w_bar, 11., 4., skel_phase, cx))
                        .child(
                            div()
                                .ml_auto()
                                .child(pro_skeleton_bar(48., 10., 4., skel_phase, cx)),
                        ),
                );
            }
            return card.child(skel_list);
        }

        let mut list = div().mx(px(-6.)).flex().flex_col().gap(px(2.));
        for (ci, chat) in self.recent_chats.iter().enumerate() {
            let cid = chat.id.clone();
            let ctitle = chat.title.clone();
            list = list.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("rchat-{ci}")))
                    .role(Role::Button)
                    .tab_index(0)
                    .w_full()
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(8.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(18.))
                            .flex_none()
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 24 24", CHAT_DOTS_SVG, 12., p.muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .child(ctitle),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(chat.updated_label.clone()),
                    ),
                cx,
                move |_, _, cx| {
                    cx.emit(AssistantOverviewEvent::OpenChat(cid.clone()));
                    cx.emit(AssistantOverviewEvent::Navigate("Chat".into()));
                },
            ));
        }
        card.child(list)
    }

    fn render_usage_card(&self, skel_phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let over = self.usage.is_over_limit();
        let share = self.usage.share();
        let bar_color: Hsla = if over {
            p.red_ink
        } else {
            rgb(0x0190fe).into()
        };

        let card = div()
            .min_w_0()
            .px(px(12.))
            .py(px(10.))
            .rounded(px(12.))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(pro_svg("0 0 24 24", USAGE_TREND_SVG, 14., p.muted))
                    .child(tr(cx, "Usage", "额度用量")),
            );

        if self.loading {
            return card.child(
                div()
                    .mt_auto()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(112., 18., 5., skel_phase, cx))
                    .child(pro_skeleton_bar(124., 4., 2., skel_phase, cx))
                    .child(pro_skeleton_bar(72., 11., 4., skel_phase, cx)),
            );
        }

        let foot_text: SharedString = if over {
            format!(
                "{} · {}",
                tr(cx, "Over limit", "已超出额度"),
                self.usage.resets_label
            )
            .into()
        } else {
            self.usage.resets_label.clone()
        };

        card.child(
            div()
                .mt_auto()
                .flex()
                .items_baseline()
                .flex_wrap()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(18.))
                        .line_height(px(24.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(format_thousands(self.usage.used_credits)),
                )
                .child(div().text_size(px(12.)).text_color(p.muted).child(format!(
                    "/ {} {}",
                    format_thousands(self.usage.limit_credits),
                    tr(cx, "credits", "积分")
                ))),
        )
        .child(pro_spring_bar(
            pro_eid(&self.id, "ov-usage-bar"),
            share,
            4.,
            2.,
            p.chip,
            bar_color,
        ))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(if over { p.red_ink } else { p.muted })
                .child(foot_text),
        )
    }

    fn render_files_card(&self, skel_phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let card = div()
            .min_w_0()
            .px(px(12.))
            .py(px(10.))
            .rounded(px(12.))
            .bg(p.panel)
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
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(pro_svg("0 0 24 24", FILES_HEAD_SVG, 14., p.muted))
                            .child(tr(cx, "Files made for you", "为你生成的文件")),
                    )
                    .child(if self.loading {
                        pro_skeleton_bar(42., 12., 5., skel_phase, cx).into_any_element()
                    } else {
                        activate(
                            div()
                                .id(pro_eid(&self.id, "files-more"))
                                .role(Role::Button)
                                .tab_index(0)
                                .mr(px(-6.))
                                .px(px(6.))
                                .py(px(2.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                .text_size(px(11.5))
                                .text_color(p.text)
                                .cursor_pointer()
                                .child(format!(
                                    "{} {}",
                                    self.more_files_count,
                                    tr(cx, "more", "更多")
                                )),
                            cx,
                            |_, _, cx| {
                                cx.emit(AssistantOverviewEvent::Navigate("Folders".into()));
                            },
                        )
                        .into_any_element()
                    }),
            );

        if self.loading {
            let mut skel_list = div().flex().flex_col().gap(px(6.));
            for w_bar in [128., 144., 136.] {
                skel_list = skel_list.child(
                    div()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(18., 18., 6., skel_phase, cx))
                        .child(pro_skeleton_bar(w_bar, 11., 4., skel_phase, cx))
                        .child(
                            div()
                                .ml_auto()
                                .child(pro_skeleton_bar(44., 10., 4., skel_phase, cx)),
                        ),
                );
            }
            return card.child(skel_list);
        }

        let mut list = div().mx(px(-6.)).flex().flex_col().gap(px(2.));
        for (fi, file) in self.files.iter().enumerate() {
            let (stem, ext) = split_filename_extension(&file.filename);
            let stem_s: SharedString = stem.to_owned().into();
            let ext_s: SharedString = ext.to_owned().into();
            let fname_emit = file.filename.clone();
            let (glyph_svg, glyph_color) = match file.kind {
                OverviewFileKind::Document => (FILE_DOC_GLYPH_SVG, p.blue),
                OverviewFileKind::Spreadsheet => (FILE_SHEET_GLYPH_SVG, p.green),
                OverviewFileKind::Code => (FILE_DOC_GLYPH_SVG, p.orange),
            };

            list = list.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("rfile-{fi}")))
                    .role(Role::Button)
                    .tab_index(0)
                    .w_full()
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(8.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(18.))
                            .flex_none()
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 12 12", glyph_svg, 12., glyph_color)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .child(div().min_w_0().truncate().child(stem_s))
                            .when(!ext_s.is_empty(), |d| {
                                d.child(div().flex_none().child(ext_s))
                            }),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(file.updated_label.clone()),
                    ),
                cx,
                move |this, _, cx| {
                    this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    cx.emit(AssistantOverviewEvent::OpenFile(fname_emit.clone()));
                    cx.emit(AssistantOverviewEvent::Navigate("Folders".into()));
                },
            ));
        }
        card.child(list)
    }

    fn render_this_month_card(&self, skel_phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let card = div()
            .min_w_0()
            .px(px(12.))
            .py(px(10.))
            .rounded(px(12.))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(pro_icon(ProIcon::Calendar, 14., p.muted))
                    .child(tr(cx, "This month", "本月统计")),
            );

        if self.loading {
            return card.child(
                div()
                    .mt_auto()
                    .flex()
                    .items_end()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(42., 10., 4., skel_phase, cx))
                            .child(pro_skeleton_bar(56., 18., 5., skel_phase, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(54., 10., 4., skel_phase, cx))
                            .child(pro_skeleton_bar(48., 18., 5., skel_phase, cx)),
                    ),
            );
        }

        let fmt_delta = |delta: i32| -> (String, Hsla) {
            if delta > 0 {
                (format!("+{delta}"), p.green_ink)
            } else if delta < 0 {
                (format!("−{}", delta.unsigned_abs()), p.red_ink)
            } else {
                ("0".to_owned(), p.muted)
            }
        };
        let (chat_d_txt, chat_d_c) = fmt_delta(self.monthly.chats_delta);
        let (file_d_txt, file_d_c) = fmt_delta(self.monthly.files_delta);

        card.child(
            div()
                .mt_auto()
                .flex()
                .items_start()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.muted)
                                .truncate()
                                .child(tr(cx, "Chats", "对话")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .flex_wrap()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(18.))
                                        .line_height(px(24.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(self.monthly.chats_count.to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(chat_d_c)
                                        .child(chat_d_txt),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.muted)
                                .truncate()
                                .child(tr(cx, "Files made", "生成文件")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .flex_wrap()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(18.))
                                        .line_height(px(24.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(self.monthly.files_count.to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(file_d_c)
                                        .child(file_d_txt),
                                ),
                        ),
                ),
        )
    }

    fn render_board(&self, board_w: f32, skel_phase: f32, cx: &Context<Self>) -> Div {
        if self.empty {
            return div().w_full().child(pro_empty_state(
                ProIcon::UsageChart,
                tr(cx, "Nothing here yet", "暂无工作区数据"),
                tr(
                    cx,
                    "Your daily activity, recent chats, credit usage, and generated files will appear here once you start working.",
                    "开始对话后，你的每日活跃度、最近会话、额度用量与生成文件都会显示在这里。",
                ),
                Some((
                    pro_eid(&self.id, "ov-empty-start"),
                    tr(cx, "Start a chat", "开始对话").into(),
                    Box::new(|this: &mut Self, w, cx| {
                        this.empty = false;
                        if let Some(ed) = &this.composer_editor {
                            { let fh = ed.read(cx).focus.clone(); fh.focus(w, cx); }
                        }
                        cx.notify();
                    }),
                )),
                cx,
            ));
        }

        if board_w < 420. {
            let act_inner = (board_w - 24.).max(180.);
            return div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(self.render_activity_card(act_inner, skel_phase, cx))
                .child(self.render_recent_chats_card(skel_phase, cx))
                .child(self.render_usage_card(skel_phase, cx))
                .child(self.render_files_card(skel_phase, cx))
                .child(self.render_this_month_card(skel_phase, cx));
        }

        if board_w < 640. {
            let act_inner = (board_w - 24.).max(220.);
            return div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(self.render_activity_card(act_inner, skel_phase, cx))
                .child(self.render_recent_chats_card(skel_phase, cx))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(self.render_usage_card(skel_phase, cx)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(self.render_this_month_card(skel_phase, cx)),
                        ),
                )
                .child(self.render_files_card(skel_phase, cx));
        }

        let half_w = ((board_w - 8.) * 0.5 - 24.).max(220.);
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_stretch()
                    .gap(px(8.))
                    .child(
                        div().flex_1().min_w_0().flex().child(
                            self.render_activity_card(half_w, skel_phase, cx)
                                .size_full(),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .child(self.render_recent_chats_card(skel_phase, cx).size_full()),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_stretch()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .child(self.render_usage_card(skel_phase, cx).size_full()),
                    )
                    .child(
                        div()
                            .flex_basis(relative(0.5))
                            .flex_none()
                            .min_w_0()
                            .flex()
                            .child(self.render_files_card(skel_phase, cx).size_full()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .child(self.render_this_month_card(skel_phase, cx).size_full()),
                    ),
            )
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
            let pad_x = if narrow { 12. } else { 24. };
            let board_w = (page_w - pad_x * 2.).clamp(260., 880.);
            let mut brand_row = div()
                .flex()
                .items_center()
                .gap(px(9.))
                .text_size(px(22.))
                .line_height(px(28.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.ink);
            if let Some(mark) = pro_app_artwork("kense", 22., cx) {
                brand_row = brand_row.child(mark);
            }
            brand_row = brand_row.child(self.brand_title.clone());

            div()
                .id(pro_eid(&self.id, "ov-scroll"))
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
                        .max_w(px(880. + pad_x * 2.))
                        .px(px(pad_x))
                        .py(px(if narrow { 40. } else { 56. }))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(brand_row)
                        .child(
                            div()
                                .mt(px(if narrow { 22. } else { 30. }))
                                .w_full()
                                .max_w(px(672.))
                                .child(self.render_composer(
                                    compact_composer,
                                    elapsed_ms,
                                    skel_phase,
                                    cx,
                                )),
                        )
                        .child(div().mt(px(28.)).w_full().child(pro_view_transition(
                            pro_eid(&self.id, "ov-board-vtx"),
                            if self.loading {
                                "loading"
                            } else if self.empty {
                                "empty"
                            } else {
                                "ready"
                            },
                            self.render_board(board_w, skel_phase, cx),
                        ))),
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
                            pro_eid(&self.id, "ov-back"),
                            tr(cx, "Overview", "返回概览"),
                            Some(ProIcon::ChevronLeft),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| this.reset_conversation(cx),
                        )),
                )
                .child(
                    div()
                        .id(pro_eid(&self.id, "ov-chat-scroll"))
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
                                .children(turns_snapshot.into_iter().enumerate().map(|(ti, t)| {
                                    match t {
                                        ActiveTurn::User { text, folder } => div()
                                            .w_full()
                                            .flex()
                                            .flex_col()
                                            .items_end()
                                            .gap(px(5.))
                                            .when_some(folder, |d, fld| {
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
                                                    .child(text),
                                            ),
                                        ActiveTurn::Assistant {
                                            text,
                                            artifact,
                                            is_sheet,
                                            streaming,
                                        } => {
                                            let art_emit = artifact.clone();
                                            let glyph_svg = if is_sheet {
                                                FILE_SHEET_GLYPH_SVG
                                            } else {
                                                FILE_DOC_GLYPH_SVG
                                            };
                                            let glyph_c = if is_sheet { p.green } else { p.blue };
                                            div()
                                                .w_full()
                                                .flex()
                                                .items_start()
                                                .gap(px(12.))
                                                .child(pro_agent_avatar(
                                                    "kui-overview",
                                                    p.blue,
                                                    24.,
                                                    streaming,
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
                                                        .child(
                                                            div()
                                                                .text_size(px(13.5))
                                                                .line_height(px(21.))
                                                                .text_color(p.ink)
                                                                .child(text),
                                                        )
                                                        .child(
                                                            div().flex().child(activate(
                                                                div()
                                                                    .id(pro_eid(
                                                                        &self.id,
                                                                        format!("ov-art-{ti}"),
                                                                    ))
                                                                    .role(Role::Button)
                                                                    .tab_index(0)
                                                                    .h(px(28.))
                                                                    .pl(px(8.))
                                                                    .pr(px(10.))
                                                                    .rounded(px(7.))
                                                                    .bg(p.chip)
                                                                    .hover(move |s| {
                                                                        s.bg(p.chip_hover)
                                                                    })
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap(px(6.))
                                                                    .text_size(px(12.))
                                                                    .text_color(p.ink)
                                                                    .cursor_pointer()
                                                                    .child(pro_svg(
                                                                        "0 0 12 12",
                                                                        glyph_svg,
                                                                        12.,
                                                                        glyph_c,
                                                                    ))
                                                                    .child(artifact),
                                                                cx,
                                                                move |this, _, cx| {
                                                                    this.shell.update(
                                                                        cx,
                                                                        |s, cx| {
                                                                            s.set_panel_open(
                                                                                true, cx,
                                                                            )
                                                                        },
                                                                    );
                                                                    cx.emit(
                                                                        AssistantOverviewEvent::OpenFile(
                                                                            art_emit.clone(),
                                                                        ),
                                                                    );
                                                                },
                                                            )),
                                                        ),
                                                )
                                        }
                                    }
                                })),
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
                pro_eid(&self.id, "ov-mode-vtx"),
                if self.turns.is_empty() {
                    "overview"
                } else {
                    "chat"
                },
                body,
            ))
    }
}

impl Render for AssistantOverviewScreen {
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
        AssistantOverviewScreen, OverviewUsageStats, default_activity_days, format_thousands,
        split_filename_extension, step_heatmap_index,
    };

    #[test]
    fn activity_heatmap_sums_to_872_and_navigates_weeks_and_days() {
        let days = default_activity_days();
        assert_eq!(days.len(), 280);
        let total: u32 = days.iter().map(|d| d.chats).sum();
        assert_eq!(total, 872);

        assert_eq!(step_heatmap_index(14, 280, "left"), Some(7));
        assert_eq!(step_heatmap_index(14, 280, "right"), Some(21));
        assert_eq!(step_heatmap_index(14, 280, "up"), Some(13));
        assert_eq!(step_heatmap_index(279, 280, "down"), Some(279));
        assert_eq!(step_heatmap_index(120, 280, "home"), Some(0));
        assert_eq!(step_heatmap_index(12, 280, "end"), Some(279));
    }

    #[test]
    fn filename_extension_split_and_usage_limits_preserve_signals() {
        assert_eq!(
            split_filename_extension("q3-launch-plan.md"),
            ("q3-launch-plan", ".md")
        );
        assert_eq!(
            split_filename_extension("competitor-pricing.csv"),
            ("competitor-pricing", ".csv")
        );
        assert_eq!(split_filename_extension("Makefile"), ("Makefile", ""));
        assert_eq!(format_thousands(1_284), "1,284");

        let normal = OverviewUsageStats::default();
        assert!(!normal.is_over_limit());
        assert!((normal.share() - 0.642).abs() < 1e-3);

        let over = OverviewUsageStats {
            used_credits: 2_450,
            limit_credits: 2_000,
            resets_label: "Resets Dec 1".into(),
        };
        assert!(over.is_over_limit());
        assert_eq!(over.share(), 1.0);
        let _ = std::mem::size_of::<AssistantOverviewScreen>();
    }
}
