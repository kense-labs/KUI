//! Animated product workflow scene of a client task branching into subtasks, notes, and roles.
//!
//! Renders a `2312 : 1644` proportional stage where a client card (`Client: Unify`) flows into
//! `Task: Web Design` and branches smoothly via `#82BCFF` rounded connectors and traveling light
//! pulses into `Client Notes`, a stacked `Subtasks` deck, and assigned `Project Manager` and
//! `UX Designer` roles.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_mix_color, pro_skeleton_bar,
        pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        kense_mark,
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, OnceLock},
};
use web_time::Instant;

const STAGE_W: f32 = 2312.0;
const STAGE_H: f32 = 1644.0;
const INTRO_TOTAL_SECS: f32 = 5.60;
const PULSE_CYCLE_SECS: f32 = 3.20;

/// Interactive workflow node in [`ProjectFlow`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectFlowNode {
    /// Top client onboarding card (`Client: Unify`).
    Client,
    /// Central task card (`Task: Web Design`, default).
    #[default]
    Task,
    /// Right-hand `Client Notes` card.
    Notes,
    /// Left-hand stacked `Subtasks` cards.
    Subtasks,
    /// Assigned `Project Manager` role card.
    ProjectManager,
    /// Assigned `UX Designer` role card.
    UxDesigner,
}

impl ProjectFlowNode {
    /// All six interactive workflow nodes in topological order.
    pub const ALL: [Self; 6] = [
        Self::Client,
        Self::Task,
        Self::Notes,
        Self::Subtasks,
        Self::ProjectManager,
        Self::UxDesigner,
    ];

    /// Kebab-case slug identifier for element IDs and serialization.
    pub const fn as_slug(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Task => "task",
            Self::Notes => "notes",
            Self::Subtasks => "subtasks",
            Self::ProjectManager => "project-manager",
            Self::UxDesigner => "ux-designer",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Client => 0,
            Self::Task => 1,
            Self::Notes => 2,
            Self::Subtasks => 3,
            Self::ProjectManager => 4,
            Self::UxDesigner => 5,
        }
    }

    /// Localized short label matching [`UiLanguage`].
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Client => tr(cx, "Client", "客户"),
            Self::Task => tr(cx, "Web Design Task", "网页设计任务"),
            Self::Notes => tr(cx, "Client Notes", "客户备注"),
            Self::Subtasks => tr(cx, "Subtasks", "子任务组"),
            Self::ProjectManager => tr(cx, "Project Manager", "项目经理"),
            Self::UxDesigner => tr(cx, "UX Designer", "交互设计师"),
        }
    }
}

/// Events emitted by [`ProjectFlow`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectFlowEvent {
    /// User clicked a workflow node (or cleared selection by clicking it again / backdrop).
    SelectNode(Option<ProjectFlowNode>),
    /// User replayed the staggered build-in and subtask drag-in sequence.
    Replay,
}

impl EventEmitter<ProjectFlowEvent> for ProjectFlow {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TechTool {
    Slack,
    Gmail,
    Figma,
    Framer,
    ClickUp,
}

impl TechTool {
    const fn index(self) -> usize {
        match self {
            Self::Slack => 0,
            Self::Gmail => 1,
            Self::Figma => 2,
            Self::Framer => 3,
            Self::ClickUp => 4,
        }
    }

    fn glow_color(self, dark: bool) -> Hsla {
        match (self, dark) {
            (Self::Slack, false) => hsla(0.833, 0.58, 0.19, 0.34),
            (Self::Slack, true) => hsla(0.833, 0.58, 0.25, 0.45),
            (Self::Gmail, false) => hsla(0.0, 0.0, 0.47, 0.24),
            (Self::Gmail, true) => hsla(0.0, 0.0, 0.0, 0.45),
            (Self::Figma, false) => hsla(0.0, 0.0, 0.27, 0.30),
            (Self::Figma, true) => hsla(0.0, 0.0, 0.0, 0.45),
            (Self::Framer, false) => hsla(0.633, 0.91, 0.51, 0.34),
            (Self::Framer, true) => hsla(0.575, 0.92, 0.50, 0.40),
            (Self::ClickUp, false) => hsla(0.976, 1.0, 0.70, 0.30),
            (Self::ClickUp, true) => hsla(0.976, 0.90, 0.62, 0.32),
        }
    }

    fn backdrop_tint(self, dark: bool) -> Hsla {
        if dark {
            match self {
                Self::Slack => hsla(0.833, 0.58, 0.19, 0.28),
                Self::Gmail => hsla(0.0, 0.0, 0.60, 0.20),
                Self::Figma => hsla(0.0, 0.0, 0.49, 0.20),
                Self::Framer => hsla(0.576, 0.92, 0.50, 0.22),
                Self::ClickUp => hsla(0.0, 0.0, 0.66, 0.20),
            }
        } else {
            match self {
                Self::Slack => hsla(0.833, 0.58, 0.19, 0.10),
                Self::Gmail => rgb(0xf6f6f6).into(),
                Self::Figma => hsla(0.0, 0.0, 0.10, 0.10),
                Self::Framer => hsla(0.595, 0.85, 0.52, 0.12),
                Self::ClickUp => hsla(0.97, 0.92, 0.68, 0.13),
            }
        }
    }
}

fn tech_tool_svg(tool: TechTool) -> Arc<Image> {
    static CACHE: [OnceLock<Arc<Image>>; 5] = [const { OnceLock::new() }; 5];
    CACHE[tool.index()]
        .get_or_init(|| {
            let svg_markup = match tool {
                TechTool::Slack => r##"<svg xmlns="http://www.w3.org/2000/svg" width="108" height="108" viewBox="0 0 54 54" fill="none"><rect width="54" height="54" rx="12.73" fill="#4B144B"/><g transform="translate(12, 12.3)"><g transform="translate(0, 0)"><path d="M10.849 6.192C9.145 6.192 7.753 4.8 7.753 3.096C7.753 1.392 9.145 0 10.849 0C12.553 0 13.945 1.392 13.945 3.096V6.192H10.849Z" fill="#36C5F0"/><path d="M10.848 7.752C12.552 7.752 13.944 9.144 13.944 10.848C13.944 12.552 12.552 13.944 10.848 13.944H3.096C1.392 13.944 0 12.552 0 10.848C0 9.144 1.392 7.752 3.096 7.752H10.848Z" fill="#36C5F0"/></g><g transform="translate(15.53, 0)"><path d="M7.744 10.848C7.744 9.144 9.136 7.752 10.84 7.752C12.544 7.752 13.936 9.144 13.936 10.848C13.936 12.552 12.544 13.944 10.84 13.944H7.744V10.848Z" fill="#2EB67D"/><path d="M6.192 10.848C6.192 12.552 4.8 13.944 3.096 13.944C1.392 13.944 0 12.552 0 10.848V3.096C0 1.392 1.392 0 3.096 0C4.8 0 6.192 1.392 6.192 3.096V10.848Z" fill="#2EB67D"/></g><g transform="translate(0, 15.53)"><path d="M6.192 3.096C6.192 4.8 4.8 6.192 3.096 6.192C1.392 6.192 0 4.8 0 3.096C0 1.392 1.392 0 3.096 0H6.192V3.096Z" fill="#E01E5A"/><path d="M7.753 3.096C7.753 1.392 9.145 0 10.849 0C12.553 0 13.945 1.392 13.945 3.096V10.848C13.945 12.552 12.553 13.944 10.849 13.944C9.145 13.944 7.753 12.552 7.753 10.848V3.096Z" fill="#E01E5A"/></g><g transform="translate(15.53, 15.53)"><path d="M3.096 7.752C4.8 7.752 6.192 9.144 6.192 10.848C6.192 12.552 4.8 13.944 3.096 13.944C1.392 13.944 0 12.552 0 10.848V7.752H3.096Z" fill="#ECB22E"/><path d="M3.096 6.192C1.392 6.192 0 4.8 0 3.096C0 1.392 1.392 0 3.096 0H10.848C12.552 0 13.944 1.392 13.944 3.096C13.944 4.8 12.552 6.192 10.848 6.192H3.096Z" fill="#ECB22E"/></g></g></svg>"##,
                TechTool::Gmail => r##"<svg xmlns="http://www.w3.org/2000/svg" width="108" height="108" viewBox="0 0 54 54" fill="none"><rect width="54" height="54" rx="12.73" fill="#FFFFFF"/><g transform="translate(11.5, 14.7)"><path d="M24.181 3.154H30.84V22.429C30.84 23.59 29.898 24.532 28.737 24.532H25.232C24.651 24.532 24.181 24.061 24.181 23.48V3.154Z" fill="url(#gm0)"/><path d="M6.659 3.154H0V22.429C0 23.59 0.941 24.532 2.103 24.532H5.607C6.188 24.532 6.659 24.061 6.659 23.48V3.154Z" fill="#FC413D"/><path d="M5.472 0.781C4.064 -0.402 1.964 -0.22 0.781 1.187C-0.402 2.595 -0.22 4.695 1.187 5.878L14.518 17.084C15.04 17.522 15.8 17.522 16.322 17.084L29.653 5.878C31.06 4.695 31.242 2.595 30.059 1.187C28.876 -0.22 26.776 -0.402 25.368 0.781L15.42 9.143L5.472 0.781Z" fill="url(#gm1)"/></g><defs><linearGradient id="gm0" x1="27.51" y1="3.154" x2="27.51" y2="24.532" gradientUnits="userSpaceOnUse"><stop stop-color="#60D673"/><stop offset="0.39" stop-color="#0EBC5F"/><stop offset="0.62" stop-color="#00A9BB"/><stop offset="1" stop-color="#3186FF"/></linearGradient><linearGradient id="gm1" x1="0" y1="3.527" x2="30.84" y2="3.527" gradientUnits="userSpaceOnUse"><stop offset="0.08" stop-color="#FF63A0"/><stop offset="0.5" stop-color="#FC413D"/><stop offset="0.72" stop-color="#FC5C30"/><stop offset="0.96" stop-color="#FFDB0F"/></linearGradient></defs></svg>"##,
                TechTool::Figma => r##"<svg xmlns="http://www.w3.org/2000/svg" width="108" height="108" viewBox="0 0 54 54" fill="none"><rect width="54" height="54" rx="12.73" fill="url(#fgbg)"/><g transform="translate(15.5, 9.8)"><path d="M11.41 17.118C11.41 13.998 13.939 11.47 17.058 11.47C20.177 11.47 22.706 13.998 22.706 17.118C22.706 20.237 20.177 22.765 17.058 22.765C13.939 22.765 11.41 20.237 11.41 17.118Z" fill="#1ABCFE"/><path d="M0.115 28.413C0.115 25.294 2.644 22.765 5.763 22.765H11.411V28.413C11.411 31.532 8.882 34.061 5.763 34.061C2.644 34.061 0.115 31.532 0.115 28.413Z" fill="#0ACF83"/><path d="M11.41 0.174V11.47H17.058C20.177 11.47 22.706 8.941 22.706 5.822C22.706 2.703 20.177 0.174 17.058 0.174H11.41Z" fill="#FF7262"/><path d="M0.115 5.822C0.115 8.941 2.644 11.47 5.763 11.47H11.411V0.174H5.763C2.644 0.174 0.115 2.703 0.115 5.822Z" fill="#F24E1E"/><path d="M0.115 17.118C0.115 20.237 2.644 22.765 5.763 22.765H11.411V11.47H5.763C2.644 11.47 0.115 13.998 0.115 17.118Z" fill="#A259FF"/></g><defs><linearGradient id="fgbg" x1="27" y1="0" x2="27" y2="54" gradientUnits="userSpaceOnUse"><stop stop-color="#292C2E"/><stop offset="1" stop-color="#050505"/></linearGradient></defs></svg>"##,
                TechTool::Framer => r##"<svg xmlns="http://www.w3.org/2000/svg" width="108" height="108" viewBox="0 0 54 54" fill="none"><rect width="54" height="54" rx="12.73" fill="url(#frbg)"/><path d="M16 11.5H38V22H27L16 11.5ZM16 22H27L38 32.5H16V22ZM16 32.5H27V43L16 32.5Z" fill="#FFFFFF"/><defs><linearGradient id="frbg" x1="27" y1="0" x2="27" y2="54" gradientUnits="userSpaceOnUse"><stop stop-color="#23CFFD"/><stop offset="0.52" stop-color="#147CEB"/><stop offset="1" stop-color="#204ED7"/></linearGradient></defs></svg>"##,
                TechTool::ClickUp => r##"<svg xmlns="http://www.w3.org/2000/svg" width="108" height="108" viewBox="0 0 54 54" fill="none"><rect width="54" height="54" rx="12.73" fill="#FFFFFF"/><g transform="translate(13.5, 11)"><path fill-rule="evenodd" clip-rule="evenodd" d="M0 24.608L4.976 20.815C7.637 24.219 10.445 25.824 13.549 25.824C16.653 25.824 19.412 24.268 21.925 20.863L27 24.559C23.354 29.422 18.821 32 13.549 32C8.327 32 3.745 29.422 0 24.608Z" fill="url(#cu0)"/><path fill-rule="evenodd" clip-rule="evenodd" d="M13.549 8.219L4.681 15.757L0.591 11.04L13.599 0L26.507 11.04L22.369 15.708L13.549 8.219Z" fill="url(#cu1)"/></g><defs><linearGradient id="cu0" x1="0" y1="26.413" x2="27.022" y2="26.413" gradientUnits="userSpaceOnUse"><stop stop-color="#8930FD"/><stop offset="1" stop-color="#49CCF9"/></linearGradient><linearGradient id="cu1" x1="0.589" y1="7.876" x2="26.48" y2="7.876" gradientUnits="userSpaceOnUse"><stop stop-color="#FF02F0"/><stop offset="1" stop-color="#FFC800"/></linearGradient></defs></svg>"##,
            };
            Arc::new(Image::from_bytes(ImageFormat::Svg, svg_markup.as_bytes().to_vec()))
        })
        .clone()
}

fn hand_cursor_svg(open: bool) -> Arc<Image> {
    static GRAB: OnceLock<Arc<Image>> = OnceLock::new();
    static OPEN: OnceLock<Arc<Image>> = OnceLock::new();
    if open {
        OPEN.get_or_init(|| {
            let s = r##"<svg xmlns="http://www.w3.org/2000/svg" width="174" height="174" viewBox="0 0 32 32" fill="none"><path fill="#FFFFFF" stroke="#000000" stroke-width="0.75" stroke-linecap="round" stroke-linejoin="round" d="M12.6 16.6c-0.1-0.4-0.2-0.8-0.4-1.6c-0.2-0.6-0.3-0.9-0.5-1.2c-0.2-0.5-0.3-0.7-0.5-1.2c-0.1-0.3-0.4-1-0.5-1.4c-0.1-0.5 0-0.9 0.2-1.2c0.3-0.3 1-0.5 1.4-0.4c0.4 0.1 0.7 0.5 0.9 0.8c0.3 0.5 0.4 0.6 0.7 1.5c0.4 1 0.6 1.9 0.6 2.2l0.1 0.5c0 0 0-1.1 0-1.2c0-1-0.1-1.8 0-2.9c0-0.1 0.1-0.6 0.1-0.7c0.1-0.5 0.3-0.8 0.7-1c0.4-0.2 0.9-0.2 1.4 0c0.4 0.2 0.6 0.5 0.7 1c0 0.1 0.1 1 0.1 1.1c0 1 0 1.6 0 2.2c0 0.2 0 1.6 0 1.5c0.1-0.7 0.1-3.2 0.3-3.9c0.1-0.4 0.4-0.7 0.8-0.9c0.4-0.2 1.1-0.1 1.4 0.2c0.3 0.3 0.4 0.7 0.5 1.2c0 0.4 0 0.9 0 1.2c0 0.9 0 1.3 0 2.1c0 0 0 0.3 0 0.2c0.1-0.3 0.2-0.5 0.3-0.7c0-0.1 0.2-0.6 0.4-0.9c0.1-0.2 0.2-0.4 0.4-0.7c0.2-0.3 0.4-0.4 0.7-0.6c0.5-0.2 1.1 0.1 1.3 0.6c0.1 0.2 0 0.7 0 1.1c-0.1 0.6-0.3 1.3-0.4 1.6c-0.1 0.4-0.3 1.2-0.3 1.6c-0.1 0.4-0.2 1.4-0.4 1.8c-0.1 0.3-0.4 1-0.7 1.4c0 0-1.1 1.2-1.2 1.8c-0.1 0.6-0.1 0.6-0.1 1c0 0.4 0.1 0.9 0.1 0.9s-0.8 0.1-1.2 0c-0.4-0.1-0.9-0.8-1-1.1c-0.2-0.3-0.5-0.3-0.7 0c-0.2 0.4-0.7 1.1-1.1 1.1c-0.7 0.1-2.1 0-3.1 0c0 0 0.2-1-0.2-1.4c-0.3-0.3-0.8-0.8-1.1-1.1l-0.8-0.9c-0.3-0.4-0.6-1.1-1.2-2c-0.3-0.5-1-1.1-1.3-1.6c-0.2-0.4-0.3-1-0.2-1.3c0.2-0.6 0.7-0.9 1.4-0.8c0.5 0 0.8 0.2 1.2 0.5c0.2 0.2 0.6 0.5 0.8 0.7c0.2 0.2 0.2 0.3 0.4 0.5C12.6 16.8 12.6 16.9 12.6 16.6z"/><line stroke="#000000" stroke-width="0.75" stroke-linecap="round" x1="19.6" y1="20.7" x2="19.6" y2="17.3"/><line stroke="#000000" stroke-width="0.75" stroke-linecap="round" x1="17.6" y1="20.7" x2="17.5" y2="17.3"/><line stroke="#000000" stroke-width="0.75" stroke-linecap="round" x1="15.6" y1="17.3" x2="15.6" y2="20.7"/></svg>"##;
            Arc::new(Image::from_bytes(ImageFormat::Svg, s.as_bytes().to_vec()))
        })
        .clone()
    } else {
        GRAB.get_or_init(|| {
            let s = r##"<svg xmlns="http://www.w3.org/2000/svg" width="167" height="167" viewBox="0 0 167 167" fill="none"><path d="M65.756 67.844C68.365 66.8 73.062 67.322 74.627 70.454C75.671 73.063 76.715 76.716 76.715 76.194C76.715 74.107 76.715 69.932 77.237 67.844C77.759 66.279 78.802 64.713 80.89 64.191C82.456 63.669 84.021 63.669 85.587 63.669C87.152 64.191 88.718 65.235 89.762 66.279C91.849 69.41 91.849 76.194 91.849 75.672C92.371 74.107 92.371 69.41 93.415 67.322C93.937 66.279 96.024 65.235 97.068 64.713C98.634 64.191 100.721 64.191 102.287 64.713C103.331 64.713 105.418 66.279 105.94 67.322C106.984 68.888 107.506 74.107 108.027 76.194C108.027 76.716 108.549 74.107 109.593 72.541C111.681 69.41 118.987 68.366 119.509 75.672C119.509 79.325 119.509 78.804 119.509 81.413C119.509 84.022 119.509 85.588 119.509 87.675C119.509 89.763 118.987 94.46 118.465 96.547C117.943 98.113 116.377 101.766 114.812 103.853C114.812 103.853 109.071 110.116 108.549 113.247C108.027 116.378 108.027 116.378 108.027 118.466C108.027 120.553 108.549 123.163 108.549 123.163C108.549 123.163 104.374 123.685 102.287 123.163C100.199 122.641 97.59 118.988 97.068 117.422C96.024 115.857 94.459 115.857 93.415 117.422C92.371 119.51 89.762 123.163 88.196 123.163C84.543 123.685 77.237 123.163 72.018 123.163C72.018 123.163 73.062 117.944 70.974 115.857C69.409 114.291 66.799 111.682 65.234 110.116L61.059 105.419C59.493 103.332 55.84 100.722 54.796 94.982C53.752 90.285 53.752 87.675 54.796 85.588C55.84 83.5 58.449 82.457 59.493 82.457C60.537 82.457 63.146 82.457 64.19 82.979C65.234 83.5 65.756 84.022 66.799 85.066C67.843 86.632 68.365 87.675 67.843 85.588C67.321 84.022 66.277 82.457 65.756 80.369C65.234 78.282 63.668 75.672 63.668 72.541C61.059 72.541 61.581 69.41 65.756 67.844Z" fill="#FFFFFF" stroke="#000000" stroke-width="3.914" stroke-linejoin="round"/><path d="M102.287 108.028V90.284M91.85 108.028L91.328 90.284M81.412 90.284V108.028" stroke="#000000" stroke-width="3.914" stroke-linecap="round"/></svg>"##;
            Arc::new(Image::from_bytes(ImageFormat::Svg, s.as_bytes().to_vec()))
        })
        .clone()
    }
}

fn kense_client_mark(dark: bool) -> Arc<Image> {
    static LIGHT_MARK: OnceLock<Arc<Image>> = OnceLock::new();
    static DARK_MARK: OnceLock<Arc<Image>> = OnceLock::new();
    let (slot, bytes) = if dark {
        (&DARK_MARK, kense_mark::DARK)
    } else {
        (&LIGHT_MARK, kense_mark::LIGHT)
    };
    slot.get_or_init(|| Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())))
        .clone()
}

fn cubic_pt(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let b0 = u * u * u;
    let b1 = 3.0 * u * u * t;
    let b2 = 3.0 * u * t * t;
    let b3 = t * t * t;
    (
        b0 * p0.0 + b1 * p1.0 + b2 * p2.0 + b3 * p3.0,
        b0 * p0.1 + b1 * p1.1 + b2 * p2.1 + b3 * p3.1,
    )
}

#[derive(Clone, Debug)]
struct PolyConnector {
    pts: Vec<(f32, f32)>,
    cum: Vec<f32>,
    total_len: f32,
    anchor: Option<(f32, f32, f32)>,
    draw_delay: f32,
    draw_dur: f32,
    from_node: ProjectFlowNode,
    to_node: ProjectFlowNode,
    pulse_offset: f32,
}

impl PolyConnector {
    fn from_points(
        pts: Vec<(f32, f32)>,
        anchor: Option<(f32, f32, f32)>,
        draw_delay: f32,
        draw_dur: f32,
        from_node: ProjectFlowNode,
        to_node: ProjectFlowNode,
        pulse_offset: f32,
    ) -> Self {
        let mut cum = Vec::with_capacity(pts.len());
        let mut total = 0.0f32;
        cum.push(0.0);
        for i in 1..pts.len() {
            let dx = pts[i].0 - pts[i - 1].0;
            let dy = pts[i].1 - pts[i - 1].1;
            total += (dx * dx + dy * dy).sqrt();
            cum.push(total);
        }
        Self {
            pts,
            cum,
            total_len: total.max(1e-3),
            anchor,
            draw_delay,
            draw_dur,
            from_node,
            to_node,
            pulse_offset,
        }
    }

    fn touches(&self, node: ProjectFlowNode) -> bool {
        self.from_node == node || self.to_node == node
    }
}

fn build_rounded_sbend(
    x_start: f32,
    y_start: f32,
    y_mid: f32,
    x_end: f32,
    y_end: f32,
) -> Vec<(f32, f32)> {
    let r = 40.0f32;
    let dir = if x_end >= x_start { 1.0 } else { -1.0 };
    let mut pts = Vec::with_capacity(32);
    pts.push((x_start, y_start));
    pts.push((x_start, y_mid - r));
    // First 90-degree quarter-circle corner
    let k = 0.552_284_8 * r;
    let a0 = (x_start, y_mid - r);
    let a1 = (x_start, y_mid - r + k);
    let a2 = (x_start + dir * (r - k), y_mid);
    let a3 = (x_start + dir * r, y_mid);
    for step in 1..=12 {
        pts.push(cubic_pt(a0, a1, a2, a3, step as f32 / 12.0));
    }
    pts.push((x_end - dir * r, y_mid));
    // Second 90-degree quarter-circle corner
    let b0 = (x_end - dir * r, y_mid);
    let b1 = (x_end - dir * (r - k), y_mid);
    let b2 = (x_end, y_mid + r - k);
    let b3 = (x_end, y_mid + r);
    for step in 1..=12 {
        pts.push(cubic_pt(b0, b1, b2, b3, step as f32 / 12.0));
    }
    pts.push((x_end, y_end));
    pts
}

fn flow_connectors() -> &'static [PolyConnector; 5] {
    static CONNS: OnceLock<[PolyConnector; 5]> = OnceLock::new();
    CONNS.get_or_init(|| {
        [
            // 0: Client -> Task
            PolyConnector::from_points(
                vec![(1133.5, 290.0), (1133.5, 359.0)],
                Some((1133.0, 289.0, 0.45)),
                0.60,
                0.30,
                ProjectFlowNode::Client,
                ProjectFlowNode::Task,
                0.00,
            ),
            // 1: Task -> Client Notes
            PolyConnector::from_points(
                vec![(1424.0, 499.0), (1493.0, 499.0)],
                Some((1423.0, 499.0, 1.30)),
                1.50,
                0.30,
                ProjectFlowNode::Task,
                ProjectFlowNode::Notes,
                0.22,
            ),
            // 2: Task -> Assigned Roles -> Project Manager
            PolyConnector::from_points(
                build_rounded_sbend(1133.0, 647.0, 722.0, 1635.5, 911.0),
                Some((1133.0, 638.0, 1.30)),
                1.50,
                0.60,
                ProjectFlowNode::Task,
                ProjectFlowNode::ProjectManager,
                0.38,
            ),
            // 3: Project Manager -> UX Designer
            PolyConnector::from_points(
                vec![(1627.5, 1189.0), (1627.5, 1235.0)],
                Some((1627.0, 1188.0, 2.50)),
                2.65,
                0.25,
                ProjectFlowNode::ProjectManager,
                ProjectFlowNode::UxDesigner,
                0.66,
            ),
            // 4: Task -> Subtasks
            PolyConnector::from_points(
                build_rounded_sbend(1133.0, 647.0, 722.0, 615.75, 816.85),
                None,
                5.05,
                0.45,
                ProjectFlowNode::Task,
                ProjectFlowNode::Subtasks,
                0.50,
            ),
        ]
    })
}

fn dashed_drag_guide_pts() -> &'static Vec<(f32, f32)> {
    static GUIDE: OnceLock<Vec<(f32, f32)>> = OnceLock::new();
    GUIDE.get_or_init(|| {
        let mut pts = Vec::with_capacity(49);
        let ox = 662.0;
        let oy = 686.0;
        let p0 = (ox + 471.088, oy + 0.101);
        let p1 = (ox + 473.088, oy + 39.601);
        let p2 = (ox + 401.588, oy + 67.101);
        let p3 = (ox + 343.088, oy + 69.601);
        for i in 0..=24 {
            pts.push(cubic_pt(p0, p1, p2, p3, i as f32 / 24.0));
        }
        let q1 = (ox + 284.588, oy + 72.101);
        let q2 = (ox + 224.588, oy + 22.601);
        let q3 = (ox + 147.588, oy + 20.101);
        for i in 1..=16 {
            pts.push(cubic_pt(p3, q1, q2, q3, i as f32 / 16.0));
        }
        let r1 = (ox + 70.588, oy + 17.601);
        let r2 = (ox + 10.588, oy + 55.601);
        let r3 = (ox + 9.588, oy + 96.101);
        for i in 1..=16 {
            pts.push(cubic_pt(q3, r1, r2, r3, i as f32 / 16.0));
        }
        pts
    })
}

fn paint_poly_window(
    conn: &PolyConnector,
    u_start: f32,
    u_end: f32,
    stroke_px: f32,
    col: Hsla,
    map_pt: &dyn Fn(f32, f32) -> Point<Pixels>,
    window: &mut Window,
) {
    let a = (u_start.clamp(0.0, 1.0)) * conn.total_len;
    let b = (u_end.clamp(0.0, 1.0)) * conn.total_len;
    if b - a <= 0.25 || conn.pts.len() < 2 {
        return;
    }
    let mut pb = PathBuilder::stroke(px(stroke_px));
    let mut started = false;
    for i in 0..(conn.pts.len() - 1) {
        let sa = conn.cum[i];
        let sb = conn.cum[i + 1];
        if sb <= a || sa >= b {
            continue;
        }
        let seg = (sb - sa).max(1e-4);
        let t0 = ((a - sa) / seg).clamp(0.0, 1.0);
        let t1 = ((b - sa) / seg).clamp(0.0, 1.0);
        let x0 = conn.pts[i].0 + (conn.pts[i + 1].0 - conn.pts[i].0) * t0;
        let y0 = conn.pts[i].1 + (conn.pts[i + 1].1 - conn.pts[i].1) * t0;
        let x1 = conn.pts[i].0 + (conn.pts[i + 1].0 - conn.pts[i].0) * t1;
        let y1 = conn.pts[i].1 + (conn.pts[i + 1].1 - conn.pts[i].1) * t1;
        if !started {
            pb.move_to(map_pt(x0, y0));
            started = true;
        }
        pb.line_to(map_pt(x1, y1));
    }
    if started && let Ok(path) = pb.build() {
        window.paint_path(path, col);
    }
}

fn ease_card(u: f32) -> f32 {
    let t = u.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[derive(Clone, Copy, Debug)]
struct IntroFrame {
    client_alpha: f32,
    client_dy: f32,
    task_alpha: f32,
    task_dy: f32,
    notes_alpha: f32,
    notes_dy: f32,
    roles_alpha: f32,
    pm_alpha: f32,
    pm_dy: f32,
    ux_alpha: f32,
    ux_dy: f32,
    sub_alpha: f32,
    sub_dx: f32,
    sub_dy: f32,
    sub_scale: f32,
    hand_alpha: f32,
    hand_dx: f32,
    hand_dy: f32,
    hand_open: bool,
    guide_head: f32,
    guide_tail: f32,
    arrow_alpha: f32,
    steady: bool,
}

fn eval_intro_frame(t_opt: Option<f32>) -> IntroFrame {
    let Some(t) = t_opt else {
        return IntroFrame {
            client_alpha: 1.0,
            client_dy: 0.0,
            task_alpha: 1.0,
            task_dy: 0.0,
            notes_alpha: 1.0,
            notes_dy: 0.0,
            roles_alpha: 1.0,
            pm_alpha: 1.0,
            pm_dy: 0.0,
            ux_alpha: 1.0,
            ux_dy: 0.0,
            sub_alpha: 1.0,
            sub_dx: 0.0,
            sub_dy: 0.0,
            sub_scale: 1.0,
            hand_alpha: 0.0,
            hand_dx: 0.0,
            hand_dy: 0.0,
            hand_open: true,
            guide_head: 0.0,
            guide_tail: 0.0,
            arrow_alpha: 0.0,
            steady: true,
        };
    };

    let stage = |delay: f32, dur: f32| ease_card((t - delay) / dur);
    let c_u = stage(0.0, 0.55);
    let t_u = stage(0.85, 0.45);
    let n_u = stage(1.75, 0.45);
    let r_u = ((t - 1.80) / 0.40).clamp(0.0, 1.0);
    let pm_u = stage(2.10, 0.45);
    let ux_u = stage(2.90, 0.45);

    // Subtask drag-in (`3.40s .. 4.15s`) + drop bounce (`4.18s .. 4.63s`)
    let drag_raw = ((t - 3.40) / 0.75).clamp(0.0, 1.0);
    let drag_eased = ease_card(drag_raw);
    let sub_alpha = (drag_raw / 0.20).clamp(0.0, 1.0);
    let sub_dx = -300.0 * (1.0 - drag_eased);
    let sub_dy = 340.0 * (1.0 - drag_eased);
    let drop_u = ((t - 4.18) / 0.45).clamp(0.0, 1.0);
    let sub_scale = if drop_u <= 0.0 || drop_u >= 1.0 {
        1.0
    } else if drop_u < 0.35 {
        1.0 - 0.045 * (drop_u / 0.35)
    } else if drop_u < 0.70 {
        0.955 + 0.065 * ((drop_u - 0.35) / 0.35)
    } else {
        1.02 - 0.02 * ((drop_u - 0.70) / 0.30)
    };

    // Hand cursor (`3.40s .. 5.08s`)
    let away_u = ((t - 4.63) / 0.45).clamp(0.0, 1.0);
    let hand_alpha = if t < 3.40 || t >= 5.08 {
        0.0
    } else {
        sub_alpha * (1.0 - away_u * away_u)
    };
    let hand_dx = sub_dx - 150.0 * (away_u * away_u);
    let hand_dy = sub_dy + 170.0 * (away_u * away_u);
    let hand_open = t >= 4.33;

    // Dashed Bezier guide (`draw 3.40..4.15s`, `undraw 4.80..5.30s`)
    let guide_head = ease_card((t - 3.40) / 0.75);
    let guide_tail = ((t - 4.80) / 0.50).clamp(0.0, 1.0).powi(2);
    let arrow_in = ((t - 4.05) / 0.15).clamp(0.0, 1.0);
    let arrow_out = 1.0 - ((t - 5.15) / 0.25).clamp(0.0, 1.0);

    IntroFrame {
        client_alpha: c_u,
        client_dy: -34.0 * (1.0 - c_u),
        task_alpha: t_u,
        task_dy: 18.0 * (1.0 - t_u),
        notes_alpha: n_u,
        notes_dy: 18.0 * (1.0 - n_u),
        roles_alpha: r_u,
        pm_alpha: pm_u,
        pm_dy: 18.0 * (1.0 - pm_u),
        ux_alpha: ux_u,
        ux_dy: 18.0 * (1.0 - ux_u),
        sub_alpha,
        sub_dx,
        sub_dy,
        sub_scale,
        hand_alpha,
        hand_dx,
        hand_dy,
        hand_open,
        guide_head,
        guide_tail,
        arrow_alpha: arrow_in * arrow_out,
        steady: t >= INTRO_TOTAL_SECS,
    }
}

/// Animated product workflow scene of a client task branching into subtasks, notes, and roles.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::project_flow::ProjectFlow;
///
/// # fn example(cx: &mut gpui::App) {
/// let _scene = cx.new(|cx| ProjectFlow::new("workflow-scene", cx).with_client_name("Unify"));
/// # }
/// ```
pub struct ProjectFlow {
    id: ElementId,
    focus: FocusHandle,
    label: Option<SharedString>,
    client_name: SharedString,
    task_title: Option<SharedString>,
    selected_node: Option<ProjectFlowNode>,
    hovered_node: Option<ProjectFlowNode>,
    node_hover: [Slide; 6],
    node_press: [Slide; 6],
    conn_illum: [Slide; 5],
    conn_packets: [Option<Instant>; 5],
    show_replay: bool,
    playing: bool,
    started_at: Instant,
    intro_started_at: Option<Instant>,
    height: Option<Pixels>,
    loading: bool,
    empty: bool,
    measured_size: Rc<Cell<(f32, f32)>>,
}

impl ProjectFlow {
    /// Create a [`ProjectFlow`] scene ready in its full branching layout with live connector pulses.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            label: None,
            client_name: "Unify".into(),
            task_title: None,
            selected_node: None,
            hovered_node: None,
            node_hover: [
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
            ],
            node_press: [
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
            ],
            conn_illum: [
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
            ],
            conn_packets: [None; 5],
            show_replay: true,
            playing: true,
            started_at: cx.background_executor().now(),
            intro_started_at: None,
            height: None,
            loading: false,
            empty: false,
            measured_size: Rc::new(Cell::new((760.0, 540.0))),
        }
    }

    /// Override the accessible description of the scene.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Customize the client organization name shown in the top card (`"Unify"` by default).
    pub fn with_client_name(mut self, name: impl Into<SharedString>) -> Self {
        self.client_name = name.into();
        self
    }

    /// Customize the central task heading (`"Task: Web Design"` by default).
    pub fn with_task_title(mut self, title: impl Into<SharedString>) -> Self {
        self.task_title = Some(title.into());
        self
    }

    /// Pre-select a workflow node to highlight its incident branches.
    pub fn with_selected_node(mut self, node: Option<ProjectFlowNode>) -> Self {
        self.selected_node = node;
        self
    }

    /// Show or hide the interactive top-right `Replay` pill (`true` default).
    pub fn with_show_replay(mut self, show: bool) -> Self {
        self.show_replay = show;
        self
    }

    /// Automatically run the `5.6s` staggered build-in and hand drag-in sequence on mount.
    pub fn with_auto_intro(mut self, auto_intro: bool) -> Self {
        self.intro_started_at = if auto_intro {
            Some(Instant::now())
        } else {
            None
        };
        self
    }

    /// Control whether continuous light pulses travel along connectors (`true` default).
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Override the explicit container height (otherwise scales `2312 : 1644` from width).
    pub fn with_height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Replay the staggered card build-in and subtask hand drag-and-drop sequence from `t = 0`.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.intro_started_at = Some(cx.background_executor().now());
        cx.emit(ProjectFlowEvent::Replay);
        cx.notify();
    }

    fn fire_incident_packets(&mut self, node: ProjectFlowNode, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        for (ci, conn) in flow_connectors().iter().enumerate() {
            if conn.touches(node) {
                self.conn_packets[ci] = Some(now);
                self.conn_illum[ci].duration(1.0, 0.20, cx);
            }
        }
    }

    /// Return the currently selected [`ProjectFlowNode`], if any.
    pub fn selected_node(&self) -> Option<ProjectFlowNode> {
        self.selected_node
    }

    /// Programmatically select or clear a [`ProjectFlowNode`].
    pub fn select_node(&mut self, node: Option<ProjectFlowNode>, cx: &mut Context<Self>) {
        if let Some(target) = node {
            self.fire_incident_packets(target, cx);
        }
        if self.selected_node != node {
            self.selected_node = node;
            cx.emit(ProjectFlowEvent::SelectNode(node));
            cx.notify();
        }
    }

    /// Toggle continuous connector pulse playback at runtime.
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.playing != playing {
            self.playing = playing;
            cx.notify();
        }
    }

    /// Return whether continuous connector pulses are enabled.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Toggle skeleton loading mode at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether skeleton loading mode is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle the empty state at runtime.
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
}

#[allow(clippy::too_many_arguments)]
fn render_avatar_badge(
    index: usize,
    agent_badge: bool,
    bg_kind: u8,
    diameter: f32,
    dark: bool,
    pulse_phase: f32,
) -> AnyElement {
    let ring_bg: Hsla = match (bg_kind, dark) {
        (0, false) => rgb(0xfdf7e8).into(),
        (0, true) => rgb(0x352d19).into(),
        (1, false) => rgb(0xf0e8fd).into(),
        (1, true) => rgb(0x2e2653).into(),
        (_, false) => rgb(0xfde8e8).into(),
        (_, _) => rgb(0x59274b).into(),
    };
    let inner_sz = (diameter * 0.68).max(10.0);
    let inner = if agent_badge {
        let tone: Hsla = match bg_kind {
            0 => rgb(0xd99b26).into(),
            1 => rgb(0x7c5cff).into(),
            _ => rgb(0xe04f7b).into(),
        };
        let cells = agent_pixel_cells("ux-agent");
        let sq = (inner_sz * 0.22).max(2.0);
        let gap = (inner_sz * 0.08).max(0.8);
        let mut grid = div().flex().flex_col().gap(px(gap));
        for r in 0..3usize {
            let mut row_el = div().flex().items_center().gap(px(gap));
            for c in 0..3usize {
                let idx_cell = r * 3 + c;
                let wave = 0.62
                    + 0.38
                        * ((pulse_phase * 1000.0 + AGENT_WAVE_MS[idx_cell]) / 1200.0
                            * std::f32::consts::TAU)
                            .cos();
                let a = if cells[idx_cell] { wave } else { 0.14 * wave };
                row_el = row_el.child(
                    div()
                        .size(px(sq))
                        .rounded(px((sq * 0.28).max(0.6)))
                        .bg(tone.opacity(a)),
                );
            }
            grid = grid.child(row_el);
        }
        grid.into_any_element()
    } else {
        let initials = match index % 5 {
            0 => "AM",
            1 => "SL",
            2 => "PM",
            3 => "UX",
            _ => "JD",
        };
        let fg: Hsla = match (bg_kind, dark) {
            (0, false) => rgb(0x9a6412).into(),
            (0, true) => rgb(0xfbbf24).into(),
            (1, false) => rgb(0x5b21b6).into(),
            (1, true) => rgb(0xc4b5fd).into(),
            (_, false) => rgb(0xbe123c).into(),
            (_, _) => rgb(0xfda4af).into(),
        };
        div()
            .size(px(inner_sz))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px((inner_sz * 0.42).max(7.5)))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(fg)
            .child(initials)
            .into_any_element()
    };
    div()
        .size(px(diameter))
        .rounded_full()
        .bg(ring_bg)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(inner)
        .into_any_element()
}

fn render_client_mark_tile(s: f32, dark: bool) -> AnyElement {
    let outer_sz = (61.6 * s).max(14.0);
    let tile_sz = (53.8 * s).max(12.0);
    let logo_sz = (24.0 * s).max(8.0);
    let backdrop_col: Hsla = if dark {
        hsla(0.0, 0.0, 0.85, 0.20)
    } else {
        hsla(0.0, 0.0, 0.74, 0.22)
    };
    let face_bg: Hsla = if dark {
        rgb(0x292929).into()
    } else {
        rgb(0xfcfcfc).into()
    };
    let glow_col: Hsla = if dark {
        hsla(0.0, 0.0, 0.0, 0.50)
    } else {
        hsla(0.0, 0.0, 0.72, 0.36)
    };
    div()
        .relative()
        .w(px(outer_sz))
        .h(px((63.6 * s).max(14.5)))
        .flex_none()
        .child(
            div()
                .absolute()
                .left(px(4.6 * s))
                .top(px(51.0 * s))
                .w(px(tile_sz))
                .h(px((10.6 * s).max(2.5)))
                .rounded(px(12.7 * s))
                .bg(glow_col),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .size(px(outer_sz))
                .rounded(px((15.0 * s).max(4.0)))
                .bg(backdrop_col),
        )
        .child(
            div()
                .absolute()
                .left(px(3.9 * s))
                .top(px(3.9 * s))
                .size(px(tile_sz))
                .rounded(px((12.7 * s).max(3.5)))
                .bg(face_bg)
                .border_1()
                .border_color(if dark {
                    hsla(0.0, 0.0, 1.0, 0.08)
                } else {
                    rgb(0xefefef).into()
                })
                .flex()
                .items_center()
                .justify_center()
                .child(img(kense_client_mark(dark)).size(px(logo_sz)).flex_none()),
        )
        .into_any_element()
}

fn render_tech_stack_row(tools: &[TechTool], s: f32, dark: bool, cx: &App) -> AnyElement {
    let pill_bg: Hsla = if dark {
        rgb(0x352d19).into()
    } else {
        rgb(0xfdf7e8).into()
    };
    let pill_border: Hsla = if dark {
        hsla(0.119, 0.85, 0.55, 0.22)
    } else {
        rgb(0xffebb9).into()
    };
    let pill_fg: Hsla = if dark {
        rgb(0xeeb629).into()
    } else {
        rgb(0xbf9221).into()
    };

    let pill = div()
        .px(px((15.5 * s).max(5.0)))
        .py(px((11.4 * s).max(3.0)))
        .rounded_full()
        .bg(pill_bg)
        .border_1()
        .border_color(pill_border)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px((20.67 * s).max(7.5)))
                .line_height(px((20.67 * s).max(8.5)))
                .font_weight(FontWeight::MEDIUM)
                .text_color(pill_fg)
                .whitespace_nowrap()
                .child(tr(cx, "Tech-Stack", "技术栈")),
        );

    let tile_sz = (53.8 * s).max(13.0);
    let ring_pad = (5.0 * s).max(1.5);
    let outer_sz = tile_sz + ring_pad * 2.0;
    let icon_gap = if dark { 8.0 * s } else { 14.0 * s };

    let mut icons_row = div().flex().items_center().gap(px(icon_gap.max(3.0)));
    for &tool in tools {
        let img_arc = tech_tool_svg(tool);
        icons_row = icons_row.child(
            div()
                .relative()
                .w(px(outer_sz))
                .h(px(outer_sz + (4.5 * s).max(1.5)))
                .flex_none()
                .child(
                    div()
                        .absolute()
                        .left(px(ring_pad))
                        .top(px(tile_sz - 2.0 * s))
                        .w(px(tile_sz))
                        .h(px((10.0 * s).max(2.5)))
                        .rounded(px((10.0 * s).max(3.0)))
                        .bg(tool.glow_color(dark)),
                )
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .size(px(outer_sz))
                        .rounded(px((17.5 * s).max(4.5)))
                        .bg(tool.backdrop_tint(dark)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(ring_pad))
                        .top(px(ring_pad))
                        .size(px(tile_sz))
                        .rounded(px((12.7 * s).max(3.5)))
                        .overflow_hidden()
                        .shadow_xs()
                        .child(img(img_arc).size(px(tile_sz))),
                ),
        );
    }

    div()
        .flex()
        .items_center()
        .gap(px((10.0 * s).max(3.0)))
        .child(pill)
        .child(icons_row)
        .into_any_element()
}

impl Render for ProjectFlow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let live_elapsed = now.saturating_duration_since(self.started_at).as_secs_f32();

        if self.loading {
            let phase = (live_elapsed / 1.4).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .h(self.height.unwrap_or(px(460.0)))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_skeleton_bar(520.0, 360.0, 20.0, phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No workflow branches", "暂无项目分支流"),
                    tr(
                        cx,
                        "Connect a client task to subtasks, notes, and assigned roles.",
                        "将客户任务连接至子任务、备注与指派角色以展示完整流程。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let mut intro_t: Option<f32> = None;
        if let Some(t0) = self.intro_started_at {
            let age = now.saturating_duration_since(t0).as_secs_f32();
            if reduced || age >= INTRO_TOTAL_SECS {
                self.intro_started_at = None;
            } else {
                intro_t = Some(age);
            }
        }

        let intro = eval_intro_frame(intro_t);
        let animate_pulses = self.playing && !reduced && intro.steady;

        let mut packet_progs: [Option<f32>; 5] = [None; 5];
        let mut any_packet_active = false;
        for ci in 0..5 {
            if let Some(t0) = self.conn_packets[ci] {
                let age = now.saturating_duration_since(t0).as_secs_f32();
                if age < 0.88 && !reduced {
                    any_packet_active = true;
                    packet_progs[ci] = Some(pro_ease((age / 0.88).clamp(0., 1.)));
                } else {
                    self.conn_packets[ci] = None;
                }
            }
        }

        if intro_t.is_some() || animate_pulses || any_packet_active {
            w.request_animation_frame();
        }

        let pulse_phase = if animate_pulses {
            (live_elapsed / PULSE_CYCLE_SECS).fract()
        } else {
            0.42
        };

        let size_cell = self.measured_size.clone();
        let (meas_w, _) = size_cell.get();
        let auto_h = (meas_w * (STAGE_H / STAGE_W)).clamp(250.0, 660.0);
        let resolved_h = self.height.unwrap_or_else(|| px(auto_h));
        let eff_h = f32::from(resolved_h);
        let s = (meas_w / STAGE_W).min(eff_h / STAGE_H).max(0.14);
        let stage_w_px = STAGE_W * s;
        let stage_h_px = STAGE_H * s;

        let active_focus = self.hovered_node.or(self.selected_node);
        let mut node_hovs = [0.0f32; 6];
        let mut node_downs = [0.0f32; 6];
        for node in ProjectFlowNode::ALL {
            let ni = node.index();
            let target_hov = if self.hovered_node == Some(node) {
                1.0
            } else if self.selected_node == Some(node) {
                0.75
            } else {
                0.0
            };
            self.node_hover[ni].duration(target_hov, 0.18, cx);
            node_hovs[ni] = self.node_hover[ni].get(w, cx).clamp(0., 1.);
            node_downs[ni] = self.node_press[ni].get(w, cx).clamp(0., 1.);
        }

        let mut conn_illums = [0.0f32; 5];
        for (ci, conn) in flow_connectors().iter().enumerate() {
            let touched = active_focus.is_some_and(|n| conn.touches(n));
            let target_illum = if touched {
                1.0
            } else if packet_progs[ci].is_some() {
                0.75
            } else {
                0.0
            };
            self.conn_illum[ci].duration(target_illum, 0.20, cx);
            conn_illums[ci] = self.conn_illum[ci].get(w, cx).clamp(0., 1.);
        }
        let any_focus = active_focus.is_some();

        let node_opacity = |node: ProjectFlowNode, base_alpha: f32| -> f32 {
            let hov = node_hovs[node.index()];
            let focus_mul = if any_focus && active_focus != Some(node) {
                0.78 + 0.22 * hov
            } else {
                1.0
            };
            (base_alpha * focus_mul).clamp(0.0, 1.0)
        };

        let sky: Hsla = rgb(0x82bcff).into();
        let sky_bright: Hsla = if dark {
            rgb(0xdbeafe).into()
        } else {
            rgb(0x3b82f6).into()
        };
        let card_bg: Hsla = if dark {
            rgb(0x1a1a1a).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if dark {
            rgb(0x262626).into()
        } else {
            rgb(0xefeeee).into()
        };
        let title_col: Hsla = if dark {
            rgb(0xffffff).into()
        } else {
            rgb(0x1b1b1b).into()
        };
        let sub_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.60)
        } else {
            rgb(0x989898).into()
        };
        let vline_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.24)
        } else {
            rgb(0xe8e8e8).into()
        };
        let dot_col: Hsla = if dark {
            rgb(0x272727).into()
        } else {
            rgb(0xececec).into()
        };

        // Layer 0 + Layer 1: Radial dot-grid + `#82BCFF` rounded connectors + anchor rings + flowing pulses
        let connector_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8.0 || bh <= 8.0 {
                    return;
                }
                let sc = (bw / STAGE_W).min(bh / STAGE_H);
                let x0 = f32::from(bounds.left());
                let y0 = f32::from(bounds.top());
                let map_pt = |ax: f32, ay: f32| point(px(x0 + ax * sc), px(y0 + ay * sc));

                // Subtle radial dot matrix (`38` stage-unit pitch)
                let pitch = 38.0f32;
                let dot_sz = (4.0 * sc).max(1.25);
                let mut gy = 140.0f32;
                while gy <= 1520.0 {
                    let ny = (gy - STAGE_H * 0.5) / (STAGE_H * 0.46);
                    let mut gx = 160.0f32;
                    while gx <= 2160.0 {
                        let nx = (gx - STAGE_W * 0.5) / (STAGE_W * 0.47);
                        let r2 = nx * nx + ny * ny;
                        if r2 < 1.0 {
                            let falloff = (1.0 - r2.sqrt()).powf(1.25);
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(
                                        px(x0 + gx * sc - dot_sz * 0.5),
                                        px(y0 + gy * sc - dot_sz * 0.5),
                                    ),
                                    size(px(dot_sz), px(dot_sz)),
                                ),
                                dot_col.opacity(falloff * 0.92),
                            ));
                        }
                        gx += pitch;
                    }
                    gy += pitch;
                }

                // Dashed Bezier drag guide during replay (`3.40s .. 5.30s`)
                if intro.guide_head - intro.guide_tail > 0.01 {
                    let g_pts = dashed_drag_guide_pts();
                    let n_seg = g_pts.len().saturating_sub(1);
                    let i_start = ((intro.guide_tail * n_seg as f32).floor() as usize).min(n_seg);
                    let i_end = ((intro.guide_head * n_seg as f32).ceil() as usize).min(n_seg);
                    let dash_w = (4.0 * sc).max(1.2);
                    for idx in (i_start..i_end).step_by(2) {
                        let mut pb = PathBuilder::stroke(px(dash_w));
                        pb.move_to(map_pt(g_pts[idx].0, g_pts[idx].1));
                        pb.line_to(map_pt(g_pts[idx + 1].0, g_pts[idx + 1].1));
                        if let Ok(path) = pb.build() {
                            window.paint_path(path, sky.opacity(0.88));
                        }
                    }
                }
                if intro.arrow_alpha > 0.01 {
                    let mut pb = PathBuilder::stroke(px((3.4 * sc).max(1.1)));
                    pb.move_to(map_pt(680.48, 779.10));
                    pb.line_to(map_pt(670.76, 791.47));
                    pb.line_to(map_pt(663.47, 777.41));
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, sky.opacity(intro.arrow_alpha));
                    }
                }

                // 5 solid `#82BCFF` connectors + traveling pulses + white-centered anchor circles
                let base_sw = (5.0 * sc).max(1.5);
                for (ci, conn) in flow_connectors().iter().enumerate() {
                    let draw_u = match intro_t {
                        Some(t) => ease_card((t - conn.draw_delay) / conn.draw_dur),
                        None => 1.0,
                    };
                    let illum = conn_illums[ci];
                    let line_alpha = if any_focus {
                        0.42 + 0.58 * illum
                    } else {
                        0.94 + 0.06 * illum
                    };
                    let line_col = pro_mix_color(sky, sky_bright, illum * 0.35).opacity(line_alpha);

                    paint_poly_window(
                        conn,
                        0.0,
                        draw_u,
                        base_sw * (1.0 + 0.32 * illum),
                        line_col,
                        &map_pt,
                        window,
                    );

                    // Continuous traveling pulse + immediate manual packet along completed connectors
                    if intro.steady {
                        let span = if conn.total_len > 300.0 { 0.22 } else { 0.38 };
                        let mut heads: Vec<(f32, f32)> = Vec::with_capacity(2);
                        if animate_pulses {
                            heads.push((
                                (pulse_phase + conn.pulse_offset).fract(),
                                1.0 + 0.30 * illum,
                            ));
                        }
                        if let Some(pkt_u) = packet_progs[ci] {
                            heads.push(((pkt_u * (1.0 + span)).min(1.0 + span), 1.45));
                        }
                        for (head, boost) in heads {
                            let p_start = (head - span).clamp(0.0, 1.0);
                            let p_end = head.clamp(0.0, 1.0);
                            if p_end - p_start > 0.015 {
                                paint_poly_window(
                                    conn,
                                    p_start,
                                    p_end,
                                    (12.5 * sc).max(3.2),
                                    sky.opacity((0.26 * boost).min(0.62)),
                                    &map_pt,
                                    window,
                                );
                                paint_poly_window(
                                    conn,
                                    (head - span * 0.48).clamp(0.0, 1.0),
                                    p_end,
                                    (6.4 * sc).max(2.0),
                                    pro_mix_color(sky, sky_bright, 0.65)
                                        .opacity((0.76 * boost).min(1.0)),
                                    &map_pt,
                                    window,
                                );
                            }
                        }
                    }

                    // Anchor dot with white/card center and `#82BCFF` border
                    if let Some((cx_a, cy_a, dot_delay)) = conn.anchor {
                        let dot_a = match intro_t {
                            Some(t) => ((t - dot_delay) / 0.20).clamp(0.0, 1.0),
                            None => 1.0,
                        } * line_alpha;
                        if dot_a > 0.01 {
                            let r_out = ((11.5 + 1.8 * illum) * sc).max(3.5);
                            let ring_w = (3.4 * sc).max(1.25);
                            window.paint_quad(quad(
                                Bounds::new(
                                    point(px(x0 + cx_a * sc - r_out), px(y0 + cy_a * sc - r_out)),
                                    size(px(r_out * 2.0), px(r_out * 2.0)),
                                ),
                                Corners::all(px(r_out)),
                                card_bg.opacity(dot_a),
                                Edges::all(px(ring_w)),
                                sky.opacity(dot_a),
                                BorderStyle::Solid,
                            ));
                        }
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let card_r = (37.6 * s).max(8.0);
        let title_sz = (37.6 * s).max(10.0);
        let sub_sz = (26.3 * s).max(8.0);

        let card_border_for = |node: ProjectFlowNode| -> Hsla {
            let ni = node.index();
            let mix = if self.selected_node == Some(node) {
                1.0
            } else {
                (node_hovs[ni] * 0.72 + node_downs[ni] * 0.28).clamp(0., 1.)
            };
            pro_mix_color(card_border, sky, mix)
        };
        let node_dy_offset = |node: ProjectFlowNode| -> f32 {
            let ni = node.index();
            -4.5 * node_hovs[ni] * (1.0 - node_downs[ni]) + 5.0 * node_downs[ni]
        };

        let wire_node = |el: Stateful<Div>, node: ProjectFlowNode, cx: &mut Context<Self>| {
            let ni = node.index();
            el.cursor_pointer()
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    let next = if *inside { Some(node) } else { None };
                    if *inside {
                        if this.hovered_node != Some(node) {
                            this.hovered_node = Some(node);
                            this.node_hover[ni].duration(1.0, 0.18, cx);
                            this.fire_incident_packets(node, cx);
                            cx.notify();
                        }
                    } else if this.hovered_node == Some(node) {
                        this.hovered_node = next;
                        this.node_hover[ni].duration(0.0, 0.18, cx);
                        this.node_press[ni].duration(0., 0.14, cx);
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.node_press[ni].duration(1., 0.08, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.node_press[ni].duration(0., 0.14, cx);
                        cx.notify();
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.node_press[ni].duration(0., 0.14, cx);
                        cx.notify();
                    }),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = if this.selected_node == Some(node) {
                        None
                    } else {
                        Some(node)
                    };
                    if next.is_none() {
                        this.fire_incident_packets(node, cx);
                    }
                    this.select_node(next, cx);
                }))
        };

        // 1. Client Card `(807, 152, 652 × 138)`
        let client_card = wire_node(
            div()
                .id(pro_eid(&self.id, "node-client"))
                .absolute()
                .left(px(807.0 * s))
                .top(px((152.0
                    + intro.client_dy
                    + node_dy_offset(ProjectFlowNode::Client))
                    * s))
                .w(px(652.0 * s))
                .h(px(138.0 * s))
                .rounded(px(card_r))
                .bg(card_bg)
                .border_1()
                .border_color(card_border_for(ProjectFlowNode::Client))
                .shadow_sm()
                .opacity(node_opacity(ProjectFlowNode::Client, intro.client_alpha))
                .px(px(40.0 * s))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px((10.0 * s).max(3.0)))
                        .child(
                            div()
                                .text_size(px(title_sz))
                                .line_height(px(title_sz * 1.1))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(title_col)
                                .whitespace_nowrap()
                                .child(tr(cx, "Client:", "客户:")),
                        )
                        .child(render_client_mark_tile(s, dark))
                        .child(
                            div()
                                .text_size(px(title_sz))
                                .line_height(px(title_sz * 1.1))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(title_col)
                                .whitespace_nowrap()
                                .child(self.client_name.clone()),
                        ),
                )
                .child(
                    div()
                        .w(px((2.0 * s).max(1.0)))
                        .h(px(60.0 * s))
                        .bg(vline_col)
                        .flex_none(),
                )
                .child(
                    div()
                        .text_size(px(sub_sz))
                        .line_height(px(sub_sz * 1.15))
                        .text_color(sub_col)
                        .whitespace_nowrap()
                        .child(tr(cx, "Onboarded: 25 Jun", "签约：6月25日")),
                ),
            ProjectFlowNode::Client,
            cx,
        );

        // 2. Task Card `(841, 359, 584 × 281)`
        let task_heading: SharedString = self
            .task_title
            .clone()
            .unwrap_or_else(|| tr(cx, "Task: Web Design", "任务：网页设计").into());
        let task_card = wire_node(
            div()
                .id(pro_eid(&self.id, "node-task"))
                .absolute()
                .left(px(841.0 * s))
                .top(px((359.0
                    + intro.task_dy
                    + node_dy_offset(ProjectFlowNode::Task))
                    * s))
                .w(px(584.0 * s))
                .h(px(281.0 * s))
                .rounded(px(card_r))
                .bg(card_bg)
                .border_1()
                .border_color(card_border_for(ProjectFlowNode::Task))
                .shadow_md()
                .opacity(node_opacity(ProjectFlowNode::Task, intro.task_alpha))
                .px(px(42.0 * s))
                .py(px(44.0 * s))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px((12.0 * s).max(3.0)))
                        .child(
                            div()
                                .text_size(px(title_sz))
                                .line_height(px(title_sz * 1.1))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(title_col)
                                .whitespace_nowrap()
                                .child(task_heading),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px((10.0 * s).max(3.0)))
                                .child(
                                    div()
                                        .text_size(px(sub_sz))
                                        .line_height(px(sub_sz * 1.15))
                                        .text_color(sub_col)
                                        .whitespace_nowrap()
                                        .child(tr(cx, "Assigned to", "指派给")),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px((6.0 * s).max(2.0)))
                                        .child(render_avatar_badge(
                                            0,
                                            false,
                                            0,
                                            50.3 * s,
                                            dark,
                                            pulse_phase,
                                        ))
                                        .child(render_avatar_badge(
                                            1,
                                            true,
                                            1,
                                            50.3 * s,
                                            dark,
                                            pulse_phase,
                                        )),
                                ),
                        ),
                )
                .child(render_tech_stack_row(
                    &[
                        TechTool::Slack,
                        TechTool::Gmail,
                        TechTool::Figma,
                        TechTool::Framer,
                        TechTool::ClickUp,
                    ],
                    s,
                    dark,
                    cx,
                )),
            ProjectFlowNode::Task,
            cx,
        );

        // 3. Client Notes Card `(1493, 428, 615 × 138)`
        let notes_card = wire_node(
            div()
                .id(pro_eid(&self.id, "node-notes"))
                .absolute()
                .left(px(1493.0 * s))
                .top(px((428.0
                    + intro.notes_dy
                    + node_dy_offset(ProjectFlowNode::Notes))
                    * s))
                .w(px(615.0 * s))
                .h(px(138.0 * s))
                .rounded(px(card_r))
                .bg(card_bg)
                .border_1()
                .border_color(card_border_for(ProjectFlowNode::Notes))
                .shadow_sm()
                .opacity(node_opacity(ProjectFlowNode::Notes, intro.notes_alpha))
                .px(px(38.0 * s))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px((12.0 * s).max(3.0)))
                        .child(render_client_mark_tile(s, dark))
                        .child(
                            div()
                                .text_size(px(title_sz))
                                .line_height(px(title_sz * 1.1))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(title_col)
                                .whitespace_nowrap()
                                .child(tr(cx, "Client Notes", "客户备注")),
                        ),
                )
                .child(
                    div()
                        .w(px((2.0 * s).max(1.0)))
                        .h(px(60.0 * s))
                        .bg(vline_col)
                        .flex_none(),
                )
                .child(
                    div()
                        .text_size(px(sub_sz))
                        .line_height(px(sub_sz * 1.15))
                        .text_color(sub_col)
                        .whitespace_nowrap()
                        .child(tr(cx, "Updated: Today", "更新：今天")),
                ),
            ProjectFlowNode::Notes,
            cx,
        );

        // 4. Assigned Roles Label Pill `(1497, 783, 272 × 89, rx 24)`
        let roles_pill = div()
            .absolute()
            .left(px(1497.0 * s))
            .top(px(783.0 * s))
            .w(px(272.0 * s))
            .h(px(89.0 * s))
            .rounded(px((24.0 * s).max(6.0)))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .shadow_sm()
            .opacity(intro.roles_alpha)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_size(px(sub_sz))
                    .line_height(px(sub_sz * 1.15))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(sub_col)
                    .whitespace_nowrap()
                    .child(tr(cx, "Assigned Roles", "指派角色")),
            );

        // 5. Project Manager Card `(1335, 910, 584 × 281)`
        let pm_card = wire_node(
            div()
                .id(pro_eid(&self.id, "node-pm"))
                .absolute()
                .left(px(1335.0 * s))
                .top(px((910.0
                    + intro.pm_dy
                    + node_dy_offset(ProjectFlowNode::ProjectManager))
                    * s))
                .w(px(584.0 * s))
                .h(px(281.0 * s))
                .rounded(px(card_r))
                .bg(card_bg)
                .border_1()
                .border_color(card_border_for(ProjectFlowNode::ProjectManager))
                .shadow_md()
                .opacity(node_opacity(
                    ProjectFlowNode::ProjectManager,
                    intro.pm_alpha,
                ))
                .px(px(41.5 * s))
                .py(px(44.0 * s))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px((20.0 * s).max(5.0)))
                        .child(render_avatar_badge(
                            2,
                            false,
                            0,
                            106.2 * s,
                            dark,
                            pulse_phase,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px((8.0 * s).max(2.0)))
                                .child(
                                    div()
                                        .text_size(px(title_sz))
                                        .line_height(px(title_sz * 1.1))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(title_col)
                                        .whitespace_nowrap()
                                        .child(tr(cx, "Project Manager", "项目经理")),
                                )
                                .child(
                                    div()
                                        .text_size(px(sub_sz))
                                        .line_height(px(sub_sz * 1.15))
                                        .text_color(sub_col)
                                        .whitespace_nowrap()
                                        .child(tr(
                                            cx,
                                            "Managing Project Progress",
                                            "统筹项目排期与进度",
                                        )),
                                ),
                        ),
                )
                .child(render_tech_stack_row(
                    &[TechTool::Slack, TechTool::Gmail, TechTool::ClickUp],
                    s,
                    dark,
                    cx,
                )),
            ProjectFlowNode::ProjectManager,
            cx,
        );

        // 6. UX Designer Card `(1335, 1235, 584 × 281)`
        let ux_card = wire_node(
            div()
                .id(pro_eid(&self.id, "node-ux"))
                .absolute()
                .left(px(1335.0 * s))
                .top(px((1235.0
                    + intro.ux_dy
                    + node_dy_offset(ProjectFlowNode::UxDesigner))
                    * s))
                .w(px(584.0 * s))
                .h(px(281.0 * s))
                .rounded(px(card_r))
                .bg(card_bg)
                .border_1()
                .border_color(card_border_for(ProjectFlowNode::UxDesigner))
                .shadow_md()
                .opacity(node_opacity(ProjectFlowNode::UxDesigner, intro.ux_alpha))
                .px(px(41.5 * s))
                .py(px(44.0 * s))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px((20.0 * s).max(5.0)))
                        .child(render_avatar_badge(
                            3,
                            false,
                            1,
                            106.2 * s,
                            dark,
                            pulse_phase,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px((8.0 * s).max(2.0)))
                                .child(
                                    div()
                                        .text_size(px(title_sz))
                                        .line_height(px(title_sz * 1.1))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(title_col)
                                        .whitespace_nowrap()
                                        .child(tr(cx, "UX Designer", "交互设计师")),
                                )
                                .child(
                                    div()
                                        .text_size(px(sub_sz))
                                        .line_height(px(sub_sz * 1.15))
                                        .text_color(sub_col)
                                        .whitespace_nowrap()
                                        .child(tr(
                                            cx,
                                            "Designing UX Elements",
                                            "设计交互组件与原型",
                                        )),
                                ),
                        ),
                )
                .child(render_tech_stack_row(
                    &[TechTool::Slack, TechTool::Figma, TechTool::Framer],
                    s,
                    dark,
                    cx,
                )),
            ProjectFlowNode::UxDesigner,
            cx,
        );

        // 7. Subtasks Stacked Cards (`(264.75, 816.85, 702 × 340)` + `(293.34, 973.67, 644.82 × 281)`)
        let sub_sc = s * intro.sub_scale;
        let sub_origin_dx = 615.0 * (s - sub_sc);
        let sub_origin_dy = 1025.0 * (s - sub_sc);
        let sub_border = card_border_for(ProjectFlowNode::Subtasks);

        let subtasks_group = wire_node(
            div()
                .id(pro_eid(&self.id, "node-subtasks"))
                .absolute()
                .left(px(sub_origin_dx + (264.75 + intro.sub_dx) * sub_sc))
                .top(px(sub_origin_dy
                    + (816.85
                        + intro.sub_dy
                        + node_dy_offset(ProjectFlowNode::Subtasks))
                        * sub_sc))
                .w(px(702.0 * sub_sc))
                .h(px(437.82 * sub_sc))
                .opacity(node_opacity(ProjectFlowNode::Subtasks, intro.sub_alpha))
                // Back card: Subtasks: List Below
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .w(px(702.0 * sub_sc))
                        .h(px(340.0 * sub_sc))
                        .rounded(px((37.6 * sub_sc).max(8.0)))
                        .bg(card_bg)
                        .border_1()
                        .border_color(sub_border)
                        .shadow_md()
                        .px(px(43.5 * sub_sc))
                        .py(px(44.0 * sub_sc))
                        .flex()
                        .flex_col()
                        .gap(px((12.0 * sub_sc).max(3.0)))
                        .child(
                            div()
                                .text_size(px((37.6 * sub_sc).max(10.0)))
                                .line_height(px((37.6 * sub_sc * 1.1).max(11.0)))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(title_col)
                                .whitespace_nowrap()
                                .child(tr(cx, "Subtasks: List Below", "子任务：下方列表")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px((10.0 * sub_sc).max(3.0)))
                                .child(
                                    div()
                                        .text_size(px((26.3 * sub_sc).max(8.0)))
                                        .line_height(px((26.3 * sub_sc * 1.15).max(9.0)))
                                        .text_color(sub_col)
                                        .whitespace_nowrap()
                                        .child(tr(cx, "Assigned to", "指派给")),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px((6.0 * sub_sc).max(2.0)))
                                        .child(render_avatar_badge(
                                            0,
                                            false,
                                            0,
                                            50.3 * sub_sc,
                                            dark,
                                            pulse_phase,
                                        ))
                                        .child(render_avatar_badge(
                                            1,
                                            true,
                                            1,
                                            50.3 * sub_sc,
                                            dark,
                                            pulse_phase + 0.33,
                                        ))
                                        .child(render_avatar_badge(
                                            4,
                                            false,
                                            2,
                                            50.3 * sub_sc,
                                            dark,
                                            pulse_phase,
                                        )),
                                ),
                        ),
                )
                // Front overlapping card: Subtask 1: Create Wireframe
                .child(
                    div()
                        .absolute()
                        .left(px(28.59 * sub_sc))
                        .top(px(156.82 * sub_sc))
                        .w(px(644.82 * sub_sc))
                        .h(px(281.0 * sub_sc))
                        .rounded(px((37.6 * sub_sc).max(8.0)))
                        .bg(card_bg)
                        .border_1()
                        .border_color(sub_border)
                        .shadow_lg()
                        .px(px(42.0 * sub_sc))
                        .py(px(44.0 * sub_sc))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px((12.0 * sub_sc).max(3.0)))
                                .child(
                                    div()
                                        .text_size(px((37.6 * sub_sc).max(10.0)))
                                        .line_height(px((37.6 * sub_sc * 1.1).max(11.0)))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(title_col)
                                        .whitespace_nowrap()
                                        .child(tr(
                                            cx,
                                            "Subtask 1: Create Wireframe",
                                            "子任务 1：绘制线框图",
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px((10.0 * sub_sc).max(3.0)))
                                        .child(
                                            div()
                                                .text_size(px((26.3 * sub_sc).max(8.0)))
                                                .line_height(px((26.3 * sub_sc * 1.15).max(9.0)))
                                                .text_color(sub_col)
                                                .whitespace_nowrap()
                                                .child(tr(cx, "Assigned to", "指派给")),
                                        )
                                        .child(render_avatar_badge(
                                            3,
                                            false,
                                            1,
                                            50.3 * sub_sc,
                                            dark,
                                            pulse_phase,
                                        )),
                                ),
                        )
                        .child(render_tech_stack_row(
                            &[
                                TechTool::Slack,
                                TechTool::Gmail,
                                TechTool::Figma,
                                TechTool::Framer,
                            ],
                            sub_sc,
                            dark,
                            cx,
                        )),
                ),
            ProjectFlowNode::Subtasks,
            cx,
        );

        let mut stage_div = div()
            .id(pro_eid(&self.id, "stage"))
            .relative()
            .w(px(stage_w_px))
            .h(px(stage_h_px))
            .flex_none()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.selected_node.is_some() {
                        this.select_node(None, cx);
                    }
                }),
            )
            .child(connector_canvas)
            .child(roles_pill)
            .child(client_card)
            .child(task_card)
            .child(notes_card)
            .child(pm_card)
            .child(ux_card)
            .child(subtasks_group);

        if intro.hand_alpha > 0.01 {
            let hand_sz = (168.0 * s).max(24.0);
            stage_div = stage_div.child(
                div()
                    .absolute()
                    .left(px((759.0 + intro.hand_dx) * s))
                    .top(px((762.73 + intro.hand_dy) * s))
                    .size(px(hand_sz))
                    .opacity(intro.hand_alpha)
                    .child(img(hand_cursor_svg(intro.hand_open)).size(px(hand_sz))),
            );
        }

        if self.show_replay {
            let rep_bg: Hsla = if dark {
                rgb(0x202023).into()
            } else {
                rgb(0xffffff).into()
            };
            let rep_border: Hsla = if dark {
                hsla(0.0, 0.0, 1.0, 0.08)
            } else {
                rgb(0xe8e8e8).into()
            };
            let rep_fg: Hsla = if dark {
                rgb(0xd4d4d8).into()
            } else {
                rgb(0x52525b).into()
            };
            let replay_btn = div()
                .absolute()
                .right(px(12.0))
                .top(px(12.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .child(pro_tactile(
                    pro_eid(&self.id, "replay-btn"),
                    tr(cx, "Replay workflow animation", "重播流程动画"),
                    9999.,
                    rep_bg,
                    pro_mix_color(rep_bg, sky, 0.16),
                    rep_border,
                    sky.opacity(0.65),
                    false,
                    div()
                        .h(px(26.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(pro_icon(ProIcon::Refresh, 12.0, rep_fg))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .line_height(px(13.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rep_fg)
                                .whitespace_nowrap()
                                .child(tr(cx, "Replay", "重播")),
                        ),
                    cx,
                    |this, _, cx| {
                        this.replay(cx);
                    },
                ));
            stage_div = stage_div.child(replay_btn);
        }

        let measure_sensor = canvas(
            move |bounds, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw > 24.0 && bh > 24.0 {
                    let prev = size_cell.get();
                    if (bw - prev.0).abs() > 1.0 || (bh - prev.1).abs() > 1.0 {
                        size_cell.set((bw, bh));
                        window.request_animation_frame();
                    }
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .inset_0();

        let aria_label: SharedString = self.label.clone().unwrap_or_else(|| {
            tr(
                cx,
                "A client task branching into subtasks, client notes, and assigned roles",
                "客户任务分支至子任务、客户备注与指派角色的产品流程图",
            )
            .into()
        });

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Image)
            .aria_label(aria_label)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "r" | "enter" | "space" => {
                        this.replay(cx);
                    }
                    "right" | "down" => {
                        let next = match this.selected_node {
                            None => ProjectFlowNode::Client,
                            Some(cur) => {
                                let idx = ProjectFlowNode::ALL
                                    .iter()
                                    .position(|&n| n == cur)
                                    .unwrap_or(0);
                                ProjectFlowNode::ALL[(idx + 1) % ProjectFlowNode::ALL.len()]
                            }
                        };
                        this.select_node(Some(next), cx);
                    }
                    "left" | "up" => {
                        let prev = match this.selected_node {
                            None => ProjectFlowNode::UxDesigner,
                            Some(cur) => {
                                let idx = ProjectFlowNode::ALL
                                    .iter()
                                    .position(|&n| n == cur)
                                    .unwrap_or(0);
                                let p_idx = (idx + ProjectFlowNode::ALL.len() - 1)
                                    % ProjectFlowNode::ALL.len();
                                ProjectFlowNode::ALL[p_idx]
                            }
                        };
                        this.select_node(Some(prev), cx);
                    }
                    "escape" => {
                        this.select_node(None, cx);
                    }
                    _ => {}
                }
            }))
            .relative()
            .w_full()
            .h(resolved_h)
            .flex()
            .items_center()
            .justify_center()
            .child(measure_sensor)
            .child(stage_div)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        INTRO_TOTAL_SECS, ProjectFlow, ProjectFlowEvent, ProjectFlowNode, STAGE_H, STAGE_W,
        dashed_drag_guide_pts, eval_intro_frame, flow_connectors,
    };
    use gpui::{
        MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, point, px, size,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[test]
    fn connectors_and_dashed_drag_guide_cover_all_six_workflow_nodes() {
        let conns = flow_connectors();
        assert_eq!(conns.len(), 5);
        for c in conns {
            assert!(c.pts.len() >= 2);
            assert!(c.total_len > 40.0);
        }
        for node in ProjectFlowNode::ALL {
            assert!(!node.as_slug().is_empty());
            assert!(conns.iter().any(|c| c.touches(node)));
        }
        let guide = dashed_drag_guide_pts();
        assert!(guide.len() > 40);
    }

    #[test]
    fn intro_timeline_starts_hidden_at_zero_and_settles_smoothly() {
        let steady = eval_intro_frame(None);
        assert!(steady.steady);
        assert_eq!(steady.client_alpha, 1.0);
        assert_eq!(steady.sub_alpha, 1.0);
        assert_eq!(steady.hand_alpha, 0.0);

        let t0 = eval_intro_frame(Some(0.0));
        assert!(!t0.steady);
        assert!(t0.client_alpha < 0.01);
        assert!(t0.sub_alpha < 0.01);

        let drag_mid = eval_intro_frame(Some(3.90));
        assert!(drag_mid.hand_alpha > 0.5);
        assert!(!drag_mid.hand_open);
        assert!(drag_mid.guide_head > 0.3);

        let drop_bounce = eval_intro_frame(Some(4.35));
        assert!(drop_bounce.hand_open);
        assert!((drop_bounce.sub_scale - 1.0).abs() > 0.001);

        let done = eval_intro_frame(Some(INTRO_TOTAL_SECS + 0.1));
        assert!(done.steady);
        assert!((done.ux_alpha - 1.0).abs() < 1e-4);
    }

    #[gpui::test]
    fn hovering_and_clicking_workflow_node_illuminates_incident_connectors_and_fires_packets(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let events: Rc<RefCell<Vec<ProjectFlowEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let ev_sink = events.clone();

        let (view, vcx) = cx.add_window_view(|_, cx| {
            let pf = ProjectFlow::new("pf-test", cx).with_playing(false);
            cx.subscribe_self(move |_, ev: &ProjectFlowEvent, _| {
                ev_sink.borrow_mut().push(ev.clone());
            })
            .detach();
            pf
        });
        vcx.simulate_resize(size(px(760.), px(540.)));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        // Compute `Client Notes` card (`(1493, 428, 615 × 138)`) center in stage coordinates
        let notes_pt = vcx.update(|_, app| {
            let (mw, _) = view.read(app).measured_size.get();
            let auto_h = (mw * (STAGE_H / STAGE_W)).clamp(250.0, 660.0);
            let s = (mw / STAGE_W).min(auto_h / STAGE_H).max(0.14);
            let ox = (mw - STAGE_W * s) * 0.5;
            let oy = (auto_h - STAGE_H * s) * 0.5;
            point(px(ox + (1493.0 + 300.0) * s), px(oy + (428.0 + 69.0) * s))
        });

        vcx.simulate_event(MouseMoveEvent {
            position: notes_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        exec.advance_clock(Duration::from_millis(160));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let pf = view.read(app);
            assert_eq!(pf.hovered_node, Some(ProjectFlowNode::Notes));
            // Connector 1 (`Task -> Client Notes`) illuminates and launches a traveling packet!
            assert!(pf.conn_packets[1].is_some());
            assert!(pf.conn_illum[1].value(app) > 0.6);
        });

        vcx.simulate_event(MouseDownEvent {
            position: notes_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            position: notes_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        vcx.run_until_parked();

        assert_eq!(
            vcx.update(|_, app| view.read(app).selected_node()),
            Some(ProjectFlowNode::Notes)
        );
        assert!(
            events
                .borrow()
                .contains(&ProjectFlowEvent::SelectNode(Some(ProjectFlowNode::Notes)))
        );
    }
}
