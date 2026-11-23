//! Agent skills catalog and management screen with usage charts and skill creator.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_eid,
            pro_empty_state, pro_icon_btn, pro_mark_selected, pro_mix_color, pro_modal_appear,
            pro_pill_indicator, pro_progress_ring, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_f32, pro_step_transition, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const INSTALL_RING_MS: f32 = 1400.;
const INSTALL_DONE_HOLD_MS: f32 = 420.;
const DRAFT_TWINKLE_MS: f32 = 650.;

const SVG_FEATHER: &str = r#"<path d="M5.07579 17C4.08939 4.54502 12.9123 1.0121 19.9734 2.22417C20.2585 6.35185 18.2389 7.89748 14.3926 8.61125C15.1353 9.38731 16.4477 10.3639 16.3061 11.5847C16.2054 12.4534 15.6154 12.8797 14.4355 13.7322C11.8497 15.6004 8.85421 16.7785 5.07579 17Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M4 22C4 15.5 7.84848 12.1818 10.5 10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_REPORT: &str = r#"<path d="M10.5 8H18.5M10.5 12H13M18.5 12H16M10.5 16H13M18.5 16H16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7 7.5H6C4.11438 7.5 3.17157 7.5 2.58579 8.08579C2 8.67157 2 9.61438 2 11.5V18C2 19.3807 3.11929 20.5 4.5 20.5C5.88071 20.5 7 19.3807 7 18V7.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M16 3.5H11C10.07 3.5 9.60504 3.5 9.22354 3.60222C8.18827 3.87962 7.37962 4.68827 7.10222 5.72354C7 6.10504 7 6.57003 7 7.5V18C7 19.3807 5.88071 20.5 4.5 20.5H16C18.8284 20.5 20.2426 20.5 21.1213 19.6213C22 18.7426 22 17.3284 22 14.5V9.5C22 6.67157 22 5.25736 21.1213 4.37868C20.2426 3.5 18.8284 3.5 16 3.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_GLOBE_SEARCH: &str = r#"<path d="M22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path><path d="M20 5.69899C19.0653 5.76636 17.8681 6.12824 17.0379 7.20277C15.5385 9.14361 14.039 9.30556 13.0394 8.65861C11.5399 7.6882 12.8 6.11636 11.0401 5.26215C9.89313 4.70542 9.73321 3.19045 10.3716 2" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M2 11C2.7625 11.6621 3.83046 12.2682 5.08874 12.2682C7.68843 12.2682 8.20837 12.7649 8.20837 14.7518C8.20837 16.7387 8.20837 16.7387 8.72831 18.2288C9.06651 19.1981 9.18472 20.1674 8.5106 21" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M19.8988 19.9288L22 22M21.1083 17.0459C21.1083 19.2805 19.2932 21.0919 17.0541 21.0919C14.8151 21.0919 13 19.2805 13 17.0459C13 14.8114 14.8151 13 17.0541 13C19.2932 13 21.1083 14.8114 21.1083 17.0459Z" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path>"#;

const SVG_PULL_REQUEST: &str = r#"<path d="M6 8L6 16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M18 16V12C18 9.17156 18 7.75735 17.1213 6.87867C16.2426 5.99999 14.8284 5.99999 12 5.99999L11 5.99999M11 5.99999C11 5.29976 12.9943 3.99152 13.5 3.49999M11 5.99999C11 6.70022 12.9943 8.00846 13.5 8.49999" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><circle cx="6" cy="18" r="2" stroke="currentColor" stroke-width="1.9"></circle><circle cx="6" cy="6" r="2" stroke="currentColor" stroke-width="1.9"></circle><circle cx="18" cy="18" r="2" stroke="currentColor" stroke-width="1.9"></circle>"#;

const SVG_CHART_NODES: &str = r#"<path d="M4 15L4 19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 9L12 19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M22 22L2 22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M20 13L20 19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M5.60009 8.79993C5.23521 8.31419 4.6543 8 4 8C2.89543 8 2 8.89543 2 10C2 11.1046 2.89543 12 4 12C5.10457 12 6 11.1046 6 10C6 9.54973 5.8512 9.13421 5.60009 8.79993ZM5.60009 8.79993L10.3999 5.20007M10.3999 5.20007C10.7648 5.68581 11.3457 6 12 6C12.783 6 13.4609 5.55006 13.7892 4.89462M10.3999 5.20007C10.1488 4.86579 10 4.45027 10 4C10 2.89543 10.8954 2 12 2C13.1046 2 14 2.89543 14 4C14 4.32158 13.9241 4.62543 13.7892 4.89462M13.7892 4.89462L18.2108 7.10538M18.2108 7.10538C18.0759 7.37457 18 7.67842 18 8C18 9.10457 18.8954 10 20 10C21.1046 10 22 9.10457 22 8C22 6.89543 21.1046 6 20 6C19.217 6 18.5391 6.44994 18.2108 7.10538Z" stroke="currentColor" stroke-width="1.9"></path>"#;

const SVG_PDF: &str = r#"<path d="M20 13V10.6569C20 9.83935 20 9.4306 19.8478 9.06306C19.6955 8.69552 19.4065 8.40649 18.8284 7.82843L14.0919 3.09188C13.593 2.593 13.3436 2.34355 13.0345 2.19575C12.9702 2.165 12.9044 2.13772 12.8372 2.11401C12.5141 2 12.1614 2 11.4558 2C8.21082 2 6.58831 2 5.48933 2.88607C5.26731 3.06508 5.06508 3.26731 4.88607 3.48933C4 4.58831 4 6.21082 4 9.45584V13M13 2.5V3C13 5.82843 13 7.24264 13.8787 8.12132C14.7574 9 16.1716 9 19 9H19.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M19.75 16H17.25C16.6977 16 16.25 16.4477 16.25 17V19M16.25 19V22M16.25 19H19.25M4.25 22V19.5M4.25 19.5V16H6C6.9665 16 7.75 16.7835 7.75 17.75C7.75 18.7165 6.9665 19.5 6 19.5H4.25ZM10.25 16H11.75C12.8546 16 13.75 16.8954 13.75 18V20C13.75 21.1046 12.8546 22 11.75 22H10.25V16Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_TRANSLATE: &str = r#"<path d="M2 12C2 13.0519 2.18046 14.0616 2.51212 15M13.0137 8.99997H21.5015M11 15H2.51212M21.5015 8.99997C20.266 5.50439 16.9323 2.99997 13.0137 2.99997C14.6146 2.99997 15.9226 6.76197 16.0091 11.5M21.5015 8.99997C21.7803 9.78864 21.9522 10.6278 22 11.5M2.51212 15C3.74763 18.4956 7.08134 21 11 21C9.45582 21 8.18412 17.4999 8.01831 13" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M2 5.29734C2 4.19897 2 3.64979 2.18738 3.22389C2.3861 2.77223 2.72861 2.40921 3.15476 2.1986C3.55661 2 4.07478 2 5.11111 2H6C7.88562 2 8.82843 2 9.41421 2.62085C10 3.2417 10 4.24095 10 6.23944V8.49851C10 9.37026 10 9.80613 9.73593 9.95592C9.47186 10.1057 9.12967 9.86392 8.4453 9.38036L8.34103 9.30669C7.84086 8.95329 7.59078 8.77658 7.30735 8.68563C7.02392 8.59468 6.72336 8.59468 6.12223 8.59468H5.11111C4.07478 8.59468 3.55661 8.59468 3.15476 8.39608C2.72861 8.18547 2.3861 7.82245 2.18738 7.37079C2 6.94489 2 6.39571 2 5.29734Z" stroke="currentColor" stroke-width="1.9"></path><path d="M22 17.2973C22 16.199 22 15.6498 21.8126 15.2239C21.6139 14.7722 21.2714 14.4092 20.8452 14.1986C20.4434 14 19.9252 14 18.8889 14H18C16.1144 14 15.1716 14 14.5858 14.6209C14 15.2417 14 16.2409 14 18.2394V20.4985C14 21.3703 14 21.8061 14.2641 21.9559C14.5281 22.1057 14.8703 21.8639 15.5547 21.3804L15.659 21.3067C16.1591 20.9533 16.4092 20.7766 16.6926 20.6856C16.9761 20.5947 17.2766 20.5947 17.8778 20.5947H18.8889C19.9252 20.5947 20.4434 20.5947 20.8452 20.3961C21.2714 20.1855 21.6139 19.8225 21.8126 19.3708C22 18.9449 22 18.3957 22 17.2973Z" stroke="currentColor" stroke-width="1.9"></path>"#;

const SVG_SLIDES: &str = r#"<path d="M2.5 11C2.5 13.8284 2.5 15.2426 3.37868 16.1213C4.25736 17 5.67157 17 8.5 17H15.5C18.3284 17 19.7426 17 20.6213 16.1213C21.5 15.2426 21.5 13.8284 21.5 11V8C21.5 5.17157 21.5 3.75736 20.6213 2.87868C19.7426 2 18.3284 2 15.5 2H8.5C5.67157 2 4.25736 2 3.37868 2.87868C2.5 3.75736 2.5 5.17157 2.5 8V11Z" stroke="currentColor" stroke-width="1.9"></path><path d="M7 22L12.0001 19L17 22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 17V22" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path>"#;

const SVG_RECEIPT: &str = r#"<path d="M4 18.6458V8.05426C4 5.20025 4 3.77325 4.87868 2.88663C5.75736 2 7.17157 2 10 2H14C16.8284 2 18.2426 2 19.1213 2.88663C20 3.77325 20 5.20025 20 8.05426V18.6458C20 20.1575 20 20.9133 19.538 21.2108C18.7831 21.6971 17.6161 20.6774 17.0291 20.3073C16.5441 20.0014 16.3017 19.8485 16.0325 19.8397C15.7417 19.8301 15.4949 19.9768 14.9709 20.3073L13.06 21.5124C12.5445 21.8374 12.2868 22 12 22C11.7132 22 11.4555 21.8374 10.94 21.5124L9.02913 20.3073C8.54415 20.0014 8.30166 19.8485 8.03253 19.8397C7.74172 19.8301 7.49493 19.9768 6.97087 20.3073C6.38395 20.6774 5.21687 21.6971 4.46195 21.2108C4 20.9133 4 20.1575 4 18.6458Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M11 11H8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M14 7L8 7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_NOTEPAD: &str = r#"<path d="M16.5 2V5M7.5 2V5M12 2V5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M13 3.5H11C7.70017 3.5 6.05025 3.5 5.02513 4.52513C4 5.55025 4 7.20017 4 10.5V15C4 18.2998 4 19.9497 5.02513 20.9749C6.05025 22 7.70017 22 11 22H13C16.2998 22 17.9497 22 18.9749 20.9749C20 19.9497 20 18.2998 20 15V10.5C20 7.20017 20 5.55025 18.9749 4.52512C17.9497 3.5 16.2998 3.5 13 3.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M8 15H12M8 11H16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SAMPLE_UPLOADED_SKILL_MD: &str = "---\nname: Customer changelog\ndescription: Turns closed customer requests into personal follow-up notes\ncategory: Writing\n---\n\n## Triggers\n- Draft customer follow-ups for v2.9\n- Who asked for dark-mode exports?\n\n## Steps\n1. Matches shipped Linear issues to linked customer threads\n2. Drafts a short personal note per account\n3. Leaves drafts in Inbox for review\n";

/// Functional category filter for [`AgentSkillsScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillCategory {
    #[default]
    Writing,
    Research,
    Engineering,
    Data,
    Documents,
}

impl SkillCategory {
    /// All selectable skill categories in tab order.
    pub const ALL: [Self; 5] = [
        Self::Writing,
        Self::Research,
        Self::Engineering,
        Self::Data,
        Self::Documents,
    ];

    /// Localized display label for this category.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Writing => tr(cx, "Writing", "写作"),
            Self::Research => tr(cx, "Research", "研究"),
            Self::Engineering => tr(cx, "Engineering", "工程"),
            Self::Data => tr(cx, "Data", "数据"),
            Self::Documents => tr(cx, "Documents", "文档"),
        }
    }
}

/// Active category tab on [`AgentSkillsScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillFilterTab {
    #[default]
    All,
    Category(SkillCategory),
}

impl SkillFilterTab {
    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Category(SkillCategory::Writing) => "writing",
            Self::Category(SkillCategory::Research) => "research",
            Self::Category(SkillCategory::Engineering) => "engineering",
            Self::Category(SkillCategory::Data) => "data",
            Self::Category(SkillCategory::Documents) => "documents",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SkillScopeTab {
    #[default]
    All,
    Installed,
    Discover,
}

impl SkillScopeTab {
    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Installed => "installed",
            Self::Discover => "discover",
        }
    }
}

/// Color tone used on a skill's icon tile and 14-day run bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillTone {
    #[default]
    Pink,
    Orange,
    Amber,
    Teal,
    Blue,
    Green,
    Purple,
}

impl SkillTone {
    fn resolve(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Pink => p.pink,
            Self::Orange => p.orange,
            Self::Amber => p.amber,
            Self::Teal => p.teal,
            Self::Blue => p.blue,
            Self::Green => p.green,
            Self::Purple => p.purple,
        }
    }
}

/// Vector glyph identifier rendered inside a skill's mark tile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillGlyph {
    #[default]
    Sparkles,
    Feather,
    Report,
    Mic,
    GlobeSearch,
    Search,
    PullRequest,
    Code,
    Filter,
    Chart,
    Pdf,
    Translate,
    Slides,
    Receipt,
    Users,
    Notes,
}

impl SkillGlyph {
    fn render(self, side: f32, color: Hsla) -> AnyElement {
        match self {
            Self::Sparkles => pro_icon(ProIcon::Sparkles, side, color),
            Self::Feather => pro_svg("0 0 24 24", SVG_FEATHER, side, color),
            Self::Report => pro_svg("0 0 24 24", SVG_REPORT, side, color),
            Self::Mic => pro_icon(ProIcon::Mic, side, color),
            Self::GlobeSearch => pro_svg("0 0 24 24", SVG_GLOBE_SEARCH, side, color),
            Self::Search => pro_icon(ProIcon::Search, side, color),
            Self::PullRequest => pro_svg("0 0 24 24", SVG_PULL_REQUEST, side, color),
            Self::Code => pro_icon(ProIcon::Code, side, color),
            Self::Filter => pro_icon(ProIcon::Filter, side, color),
            Self::Chart => pro_svg("0 0 24 24", SVG_CHART_NODES, side, color),
            Self::Pdf => pro_svg("0 0 24 24", SVG_PDF, side, color),
            Self::Translate => pro_svg("0 0 24 24", SVG_TRANSLATE, side, color),
            Self::Slides => pro_svg("0 0 24 24", SVG_SLIDES, side, color),
            Self::Receipt => pro_svg("0 0 24 24", SVG_RECEIPT, side, color),
            Self::Users => pro_icon(ProIcon::Users, side, color),
            Self::Notes => pro_svg("0 0 24 24", SVG_NOTEPAD, side, color),
        }
    }

    fn tab_icon(self) -> ProIcon {
        match self {
            Self::Mic => ProIcon::Mic,
            Self::Search | Self::GlobeSearch => ProIcon::Search,
            Self::PullRequest => ProIcon::GitBranch,
            Self::Code => ProIcon::Code,
            Self::Filter => ProIcon::Filter,
            Self::Chart => ProIcon::UsageChart,
            Self::Users => ProIcon::Users,
            Self::Pdf | Self::Report | Self::Receipt => ProIcon::PageDoc,
            Self::Notes => ProIcon::Note,
            Self::Feather | Self::Translate | Self::Slides | Self::Sparkles => ProIcon::Sparkles,
        }
    }
}

/// Origin provenance of an [`AgentSkill`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SkillSource {
    #[default]
    Builtin,
    Yours,
    Community(SharedString),
}

impl SkillSource {
    /// Short chip label shown on skill cards and inspector subtitles.
    pub fn badge_label(&self, cx: &App) -> SharedString {
        match self {
            Self::Builtin => tr(cx, "Built-in", "内置").into(),
            Self::Yours => tr(cx, "Yours", "自建").into(),
            Self::Community(author) => author.clone(),
        }
    }

    /// Return whether the skill can be uninstalled/removed (built-in skills can only be turned off).
    pub fn is_removable(&self) -> bool {
        !matches!(self, Self::Builtin)
    }
}

/// Recent task invocation entry shown under `Ran lately` in the skill inspector.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillRecentRun {
    pub task: SharedString,
    pub at: SharedString,
}

impl SkillRecentRun {
    /// Construct a recent skill run entry.
    pub fn new(task: impl Into<SharedString>, at: impl Into<SharedString>) -> Self {
        Self {
            task: task.into(),
            at: at.into(),
        }
    }
}

/// Agent skill definition rendered as an installed or discoverable card in [`AgentSkillsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSkill {
    pub id: SharedString,
    pub name: SharedString,
    pub description: SharedString,
    pub category: SkillCategory,
    pub glyph: SkillGlyph,
    pub tone: SkillTone,
    pub source: SkillSource,
    pub version: SharedString,
    pub installed: bool,
    pub enabled: bool,
    pub weekly_uses: u32,
    pub community_installs: Option<SharedString>,
    pub daily_runs: [u32; 14],
    pub triggers: Vec<SharedString>,
    pub steps: Vec<SharedString>,
    pub needs: Vec<SharedString>,
    pub apps: Vec<SharedString>,
    pub used_in: Vec<SkillRecentRun>,
}

impl AgentSkill {
    /// Construct an agent skill with default 14-day zeroed run bars.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
        category: SkillCategory,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category,
            glyph: SkillGlyph::Sparkles,
            tone: SkillTone::Blue,
            source: SkillSource::Yours,
            version: "v1.0".into(),
            installed: true,
            enabled: true,
            weekly_uses: 0,
            community_installs: None,
            daily_runs: [0; 14],
            triggers: Vec::new(),
            steps: Vec::new(),
            needs: Vec::new(),
            apps: Vec::new(),
            used_in: Vec::new(),
        }
    }

    /// Set the glyph and accent tone for this skill.
    pub fn with_visual(mut self, glyph: SkillGlyph, tone: SkillTone) -> Self {
        self.glyph = glyph;
        self.tone = tone;
        self
    }

    /// Set provenance and semantic version string.
    pub fn with_source(mut self, source: SkillSource, version: impl Into<SharedString>) -> Self {
        self.source = source;
        self.version = version.into();
        self
    }

    /// Configure installed and enabled flags.
    pub fn with_state(mut self, installed: bool, enabled: bool) -> Self {
        self.installed = installed;
        self.enabled = enabled;
        self
    }

    /// Set weekly run count and 14-day daily run histogram.
    pub fn with_runs(mut self, weekly_uses: u32, daily_runs: [u32; 14]) -> Self {
        self.weekly_uses = weekly_uses;
        self.daily_runs = daily_runs;
        self
    }

    /// Set the community usage string shown on Discover cards (e.g. `"8.4K use it"`).
    pub fn with_installs(mut self, installs: impl Into<SharedString>) -> Self {
        self.community_installs = Some(installs.into());
        self
    }

    /// Set example user phrases that trigger this skill.
    pub fn with_triggers<I, S>(mut self, triggers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.triggers = triggers.into_iter().map(Into::into).collect();
        self
    }

    /// Set ordered workflow steps executed by this skill.
    pub fn with_steps<I, S>(mut self, steps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.steps = steps.into_iter().map(Into::into).collect();
        self
    }

    /// Set permission capabilities required by this skill.
    pub fn with_needs<I, S>(mut self, needs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.needs = needs.into_iter().map(Into::into).collect();
        self
    }

    /// Attach connected app names rendered with [`pro_app_tile`].
    pub fn with_apps<I, S>(mut self, apps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.apps = apps.into_iter().map(Into::into).collect();
        self
    }

    /// Set recent tasks that invoked this skill.
    pub fn with_used_in(mut self, used_in: Vec<SkillRecentRun>) -> Self {
        self.used_in = used_in;
        self
    }

    /// Format this skill as a canonical `SKILL.md` document with YAML frontmatter.
    pub fn to_skill_markdown(&self) -> String {
        let cat = match self.category {
            SkillCategory::Writing => "Writing",
            SkillCategory::Research => "Research",
            SkillCategory::Engineering => "Engineering",
            SkillCategory::Data => "Data",
            SkillCategory::Documents => "Documents",
        };
        let mut out = format!(
            "---\nname: {}\ndescription: {}\ncategory: {cat}\nversion: {}\n---\n",
            self.name, self.description, self.version
        );
        if !self.triggers.is_empty() {
            out.push_str("\n## Triggers\n");
            for t in &self.triggers {
                out.push_str("- ");
                out.push_str(t);
                out.push('\n');
            }
        }
        if !self.steps.is_empty() {
            out.push_str("\n## Steps\n");
            for (i, s) in self.steps.iter().enumerate() {
                out.push_str(&format!("{}. {s}\n", i + 1));
            }
        }
        out
    }
}

/// Parse a `SKILL.md` document into a custom [`AgentSkill`].
pub fn parse_skill_markdown(markdown: &str) -> Option<AgentSkill> {
    let mut name = String::new();
    let mut description = String::new();
    let mut category = SkillCategory::Writing;
    let mut triggers = Vec::new();
    let mut steps = Vec::new();

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Section {
        Body,
        Frontmatter,
        Triggers,
        Steps,
    }

    let mut sec = Section::Body;
    let mut seen_open = false;

    for raw in markdown.lines() {
        let line = raw.trim();
        if line == "---" {
            if !seen_open {
                seen_open = true;
                sec = Section::Frontmatter;
            } else if sec == Section::Frontmatter {
                sec = Section::Body;
            }
            continue;
        }
        if sec == Section::Frontmatter {
            if let Some((k, v)) = line.split_once(':') {
                let val = v.trim().trim_matches('"');
                match k.trim().to_ascii_lowercase().as_str() {
                    "name" => name = val.to_string(),
                    "description" => description = val.to_string(),
                    "category" => {
                        category = match val.to_ascii_lowercase().as_str() {
                            "research" => SkillCategory::Research,
                            "engineering" => SkillCategory::Engineering,
                            "data" => SkillCategory::Data,
                            "documents" => SkillCategory::Documents,
                            _ => SkillCategory::Writing,
                        };
                    }
                    _ => {}
                }
            }
            continue;
        }
        if let Some(h) = line.strip_prefix("## ") {
            let hl = h.trim().to_ascii_lowercase();
            if hl.contains("trigger") || hl.contains("when") {
                sec = Section::Triggers;
            } else if hl.contains("step") || hl.contains("what") {
                sec = Section::Steps;
            } else {
                sec = Section::Body;
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        match sec {
            Section::Triggers => {
                let item = line.trim_start_matches(['-', '*', '•']).trim();
                if !item.is_empty() {
                    triggers.push(SharedString::from(item.to_string()));
                }
            }
            Section::Steps => {
                let cleaned = line
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-')
                    .trim();
                if !cleaned.is_empty() {
                    steps.push(SharedString::from(cleaned.to_string()));
                }
            }
            Section::Body | Section::Frontmatter => {
                if name.is_empty()
                    && let Some(title) = line.strip_prefix("# ")
                {
                    name = title.trim().to_string();
                } else if description.is_empty() && !line.starts_with('#') {
                    description = line.to_string();
                }
            }
        }
    }

    if name.trim().is_empty() {
        return None;
    }
    if description.is_empty() {
        description = "Custom agent workflow imported from SKILL.md".to_string();
    }
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    Some(
        AgentSkill::new(
            if slug.is_empty() {
                "custom-skill".to_string()
            } else {
                slug
            },
            name,
            description,
            category,
        )
        .with_visual(SkillGlyph::Sparkles, SkillTone::Blue)
        .with_source(SkillSource::Yours, "v1.0")
        .with_triggers(triggers)
        .with_steps(steps),
    )
}

/// Filter `(installed_indices, discover_indices)` matching category tab and search query.
pub fn filter_skills_catalog(
    skills: &[AgentSkill],
    tab: SkillFilterTab,
    query: &str,
) -> (Vec<usize>, Vec<usize>) {
    let q = query.trim().to_ascii_lowercase();
    let mut installed = Vec::new();
    let mut discover = Vec::new();

    for (idx, skill) in skills.iter().enumerate() {
        if let SkillFilterTab::Category(cat) = tab
            && skill.category != cat
        {
            continue;
        }
        if !q.is_empty() {
            let hit = skill.name.to_ascii_lowercase().contains(&q)
                || skill.description.to_ascii_lowercase().contains(&q)
                || skill
                    .triggers
                    .iter()
                    .any(|t| t.to_ascii_lowercase().contains(&q));
            if !hit {
                continue;
            }
        }
        if skill.installed {
            installed.push(idx);
        } else {
            discover.push(idx);
        }
    }
    (installed, discover)
}

/// Events emitted by [`AgentSkillsScreen`].
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentSkillsEvent {
    Select(SharedString),
    Toggle { id: SharedString, enabled: bool },
    Install(SharedString),
    Remove(SharedString),
    Create(AgentSkill),
    Navigate(SharedString),
}

impl EventEmitter<AgentSkillsEvent> for AgentSkillsScreen {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NewSkillStep {
    ChooseWay,
    Describe,
    Drafting,
    Form,
    Upload,
}

impl NewSkillStep {
    fn step_index(self) -> i32 {
        match self {
            Self::ChooseWay => 0,
            Self::Describe | Self::Upload => 1,
            Self::Drafting => 2,
            Self::Form => 3,
        }
    }
}

#[derive(Clone)]
struct InstallAnim {
    skill_id: SharedString,
    started_at: Instant,
}

/// Pro Agent Skills screen with category chips, installed/discover grid, 14-day run bars, and `New skill` modal.
pub struct AgentSkillsScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    skills: Vec<AgentSkill>,
    selected_id: Option<SharedString>,
    active_tab: SkillFilterTab,
    scope_tab: SkillScopeTab,
    search_editor: Option<Entity<Editor>>,
    installing: Option<InstallAnim>,
    fresh_landed: Option<(SharedString, Instant)>,
    modal_step: Option<NewSkillStep>,
    draft_started: Option<Instant>,
    form_category: SkillCategory,
    form_triggers: Vec<SharedString>,
    form_steps: Vec<SharedString>,
    describe_editor: Option<Entity<Editor>>,
    name_editor: Option<Entity<Editor>>,
    desc_editor: Option<Entity<Editor>>,
    show_skill_md: bool,
    loading: bool,
    empty: bool,
    standalone: bool,
    measured_width: Rc<Cell<f32>>,
}

impl Focusable for AgentSkillsScreen {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AgentSkillsScreen {
    /// Construct an [`AgentSkillsScreen`] preloaded with the 15 canonical `Fieldwork` skills.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new("thread", tr(shell_cx, "Thread", "会话"), ProIcon::Thread),
                    ShellNavItem::new("inbox", tr(shell_cx, "Inbox", "收件箱"), ProIcon::Inbox)
                        .with_count(5),
                    ShellNavItem::new("tasks", tr(shell_cx, "Tasks", "任务"), ProIcon::Tasks),
                    ShellNavItem::new("memory", tr(shell_cx, "Memory", "记忆"), ProIcon::Memory),
                    ShellNavItem::new("skills", tr(shell_cx, "Skills", "技能"), ProIcon::Sparkles),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "schedules",
                            tr(shell_cx, "Schedules", "定时计划"),
                            ProIcon::Calendar,
                        ),
                        ShellNavItem::new(
                            "activity",
                            tr(shell_cx, "Activity", "活动记录"),
                            ProIcon::Activity,
                        ),
                        ShellNavItem::new(
                            "usage",
                            tr(shell_cx, "Usage", "用量统计"),
                            ProIcon::UsageChart,
                        ),
                        ShellNavItem::new(
                            "permissions",
                            tr(shell_cx, "Permissions", "权限策略"),
                            ProIcon::Shield,
                        ),
                        ShellNavItem::new(
                            "settings",
                            tr(shell_cx, "Settings", "设置"),
                            ProIcon::Settings,
                        ),
                    ],
                ),
            ];
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_groups(groups)
                .with_active("skills")
                .with_work_area(true, true);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => cx.emit(AgentSkillsEvent::Navigate(label.clone())),
            AppShellEvent::PanelOpen(_) => cx.notify(),
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus: cx.focus_handle(),
            skills: Self::default_skills(),
            selected_id: Some("voice".into()),
            active_tab: SkillFilterTab::All,
            scope_tab: SkillScopeTab::All,
            search_editor: None,
            installing: None,
            fresh_landed: None,
            modal_step: None,
            draft_started: None,
            form_category: SkillCategory::Writing,
            form_triggers: Vec::new(),
            form_steps: Vec::new(),
            describe_editor: None,
            name_editor: None,
            desc_editor: None,
            show_skill_md: false,
            loading: false,
            empty: false,
            standalone: false,
            measured_width: Rc::new(Cell::new(752.)),
        }
    }

    /// Canonical 15 sample skills (9 installed, 6 discoverable) for `Fieldwork`.
    pub fn default_skills() -> Vec<AgentSkill> {
        vec![
            AgentSkill::new(
                "voice",
                "Brand voice",
                "Writes in Fieldwork's tone – short, warm, no jargon",
                SkillCategory::Writing,
            )
            .with_visual(SkillGlyph::Feather, SkillTone::Pink)
            .with_source(SkillSource::Yours, "v1.4")
            .with_state(true, true)
            .with_runs(46, [7, 3, 6, 7, 2, 2, 7, 3, 4, 6, 9, 9, 8, 5])
            .with_triggers([
                "Write this in our voice",
                "Draft an email to a client",
                "Make this sound like us",
            ])
            .with_steps([
                "Reads the brand guide in Drive",
                "Rewrites in short sentences, active voice",
                "Swaps jargon for plain words",
                "Checks British spelling",
            ])
            .with_needs(["Read files in Google Drive"])
            .with_used_in(vec![
                SkillRecentRun::new("Weekly update", "40 minutes ago"),
                SkillRecentRun::new("Onboarding emails", "3 hours ago"),
            ]),
            AgentSkill::new(
                "weekly-report",
                "Weekly report",
                "Turns a week of work into a one-page update",
                SkillCategory::Writing,
            )
            .with_visual(SkillGlyph::Report, SkillTone::Orange)
            .with_source(SkillSource::Builtin, "v2.1")
            .with_state(true, true)
            .with_runs(4, [1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 2, 1, 0])
            .with_triggers([
                "Write the Friday update",
                "Summarize what shipped this week",
                "Turn done tasks into a status memo",
            ])
            .with_steps([
                "Reads completed tasks, merged PRs and calendar milestones",
                "Groups highlights into Shipped, In progress and Risks",
                "Drafts a one-page memo in your workspace tone",
            ])
            .with_needs(["Read Linear issues", "Read Slack channels"])
            .with_used_in(vec![
                SkillRecentRun::new("Friday stakeholder memo", "2 days ago"),
                SkillRecentRun::new("Product sync recap", "9 days ago"),
            ]),
            AgentSkill::new(
                "meeting-notes",
                "Meeting notes",
                "Notes and next steps from a call recording",
                SkillCategory::Documents,
            )
            .with_visual(SkillGlyph::Mic, SkillTone::Amber)
            .with_source(SkillSource::Builtin, "v1.8")
            .with_state(true, false)
            .with_runs(0, [2, 1, 3, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            .with_triggers([
                "Summarize this call recording",
                "Pull action items from today's meeting",
            ])
            .with_steps([
                "Transcribes speakers with timestamps",
                "Extracts decisions and owners",
                "Files notes into the project folder",
            ])
            .with_needs(["Read Google Calendar", "Write files in Google Drive"])
            .with_used_in(vec![SkillRecentRun::new(
                "Northwind kickoff call",
                "8 days ago",
            )]),
            AgentSkill::new(
                "deep-research",
                "Deep research",
                "Reads widely, checks sources, writes a cited brief",
                SkillCategory::Research,
            )
            .with_visual(SkillGlyph::GlobeSearch, SkillTone::Teal)
            .with_source(SkillSource::Builtin, "v3.0")
            .with_state(true, true)
            .with_runs(12, [2, 1, 3, 2, 1, 0, 1, 2, 1, 3, 2, 1, 2, 1])
            .with_triggers([
                "Research how competitors price seats",
                "Write a cited brief on EU AI rules",
                "Compare these three vendors",
            ])
            .with_steps([
                "Searches primary docs, filings and benchmarks",
                "Cross-checks claims across at least two sources",
                "Writes a structured brief with inline citations",
            ])
            .with_needs(["Browse public web", "Save briefs to Workspace"])
            .with_used_in(vec![
                SkillRecentRun::new("EU data residency brief", "Yesterday"),
                SkillRecentRun::new("Pricing page teardown", "3 days ago"),
            ]),
            AgentSkill::new(
                "competitor-watch",
                "Competitor watch",
                "Spots price, product and campaign changes on rivals' sites",
                SkillCategory::Research,
            )
            .with_visual(SkillGlyph::Search, SkillTone::Blue)
            .with_source(SkillSource::Community("@lena".into()), "v1.2")
            .with_state(true, true)
            .with_runs(6, [1, 0, 1, 1, 0, 0, 1, 2, 0, 1, 1, 0, 1, 1])
            .with_triggers([
                "Check what changed on rival pricing pages",
                "Scan competitors since last Monday",
            ])
            .with_steps([
                "Captures pricing, changelog and hero copy snapshots",
                "Diffs against last week's baseline",
                "Highlights real product or price shifts, ignoring typos",
            ])
            .with_needs(["Browse public web"])
            .with_used_in(vec![
                SkillRecentRun::new("Competitor scan", "Yesterday"),
                SkillRecentRun::new("Q4 positioning review", "6 days ago"),
            ]),
            AgentSkill::new(
                "code-review",
                "Code review",
                "Reviews a pull request for bugs, tests and naming",
                SkillCategory::Engineering,
            )
            .with_visual(SkillGlyph::PullRequest, SkillTone::Green)
            .with_source(SkillSource::Builtin, "v2.4")
            .with_state(true, true)
            .with_runs(23, [4, 3, 5, 2, 4, 0, 1, 5, 4, 3, 4, 5, 1, 1])
            .with_triggers([
                "Review PR #418",
                "Check this diff for edge cases",
                "Are tests missing here?",
            ])
            .with_steps([
                "Reads the PR diff and linked issue context",
                "Flags logic bugs, race conditions and missing tests",
                "Leaves concise inline comments on GitHub",
            ])
            .with_needs(["Read & comment on GitHub PRs"])
            .with_used_in(vec![
                SkillRecentRun::new("PR #418 · Webhook retry queue", "2 hours ago"),
                SkillRecentRun::new("PR #415 · Billing currency rounding", "Yesterday"),
            ]),
            AgentSkill::new(
                "release-notes",
                "Release notes",
                "Writes release notes people outside engineering can read",
                SkillCategory::Engineering,
            )
            .with_visual(SkillGlyph::Code, SkillTone::Amber)
            .with_source(SkillSource::Yours, "v1.1")
            .with_state(true, true)
            .with_runs(2, [0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0])
            .with_triggers([
                "Draft release notes for v2.8",
                "Turn merged PRs since Tuesday into a changelog",
            ])
            .with_steps([
                "Groups merged PRs into New, Improved and Fixed",
                "Translates internal PR titles into customer outcomes",
                "Drops internal refactors and chore commits",
            ])
            .with_needs(["Read GitHub releases & PRs"])
            .with_used_in(vec![SkillRecentRun::new("Changelog v2.7.4", "4 days ago")]),
            AgentSkill::new(
                "spreadsheet-analysis",
                "Spreadsheet analysis",
                "Cleans a sheet, finds the trend and says it in a sentence",
                SkillCategory::Data,
            )
            .with_visual(SkillGlyph::Filter, SkillTone::Teal)
            .with_source(SkillSource::Builtin, "v2.0")
            .with_state(true, true)
            .with_runs(9, [1, 2, 0, 1, 2, 0, 0, 2, 1, 2, 1, 2, 0, 1])
            .with_triggers([
                "Clean up this CSV and find the trend",
                "Why did churn dip in October?",
            ])
            .with_steps([
                "Normalizes dates, currencies and blank rows",
                "Computes period-over-period deltas and outliers",
                "Writes a plain-English headline above the table",
            ])
            .with_needs(["Read & edit Google Sheets"])
            .with_used_in(vec![
                SkillRecentRun::new("Northwind numbers", "Today 07:45"),
                SkillRecentRun::new("Q3 cohort retention", "5 days ago"),
            ]),
            AgentSkill::new(
                "chart-maker",
                "Chart maker",
                "Picks the right chart for the numbers and draws it",
                SkillCategory::Data,
            )
            .with_visual(SkillGlyph::Chart, SkillTone::Purple)
            .with_source(SkillSource::Community("@mika".into()), "v1.5")
            .with_state(true, true)
            .with_runs(14, [2, 1, 2, 3, 1, 0, 1, 2, 3, 2, 2, 3, 1, 1])
            .with_triggers([
                "Plot monthly revenue vs target",
                "Turn this table into a clean chart",
            ])
            .with_steps([
                "Chooses line, bar or stacked breakdown from the series shape",
                "Labels axes, units and key callouts",
                "Exports crisp SVG for docs or slides",
            ])
            .with_needs(["Read workspace sheets"])
            .with_used_in(vec![
                SkillRecentRun::new("Board deck revenue chart", "Yesterday"),
                SkillRecentRun::new("Weekly usage digest", "4 days ago"),
            ]),
            // Discover (6)
            AgentSkill::new(
                "pdf-forms",
                "PDF forms",
                "Reads and fills in PDF forms from your documents",
                SkillCategory::Documents,
            )
            .with_visual(SkillGlyph::Pdf, SkillTone::Orange)
            .with_source(SkillSource::Community("@sam".into()), "v1.3")
            .with_state(false, false)
            .with_installs("8.4K use it")
            .with_triggers([
                "Fill out this vendor onboarding PDF",
                "Extract fields from signed W-8BEN",
            ])
            .with_steps([
                "Detects fillable fields and checkboxes in the PDF",
                "Matches values from company profile and Drive docs",
                "Flags any missing signature or tax field before saving",
            ])
            .with_needs(["Read & write files in Google Drive"]),
            AgentSkill::new(
                "translate-localize",
                "Translate & localize",
                "Translates and adapts copy for each market",
                SkillCategory::Writing,
            )
            .with_visual(SkillGlyph::Translate, SkillTone::Teal)
            .with_source(SkillSource::Builtin, "v1.9")
            .with_state(false, false)
            .with_installs("12.9K use it")
            .with_triggers([
                "Localize this landing page for DACH and Japan",
                "Translate release notes keeping product terms",
            ])
            .with_steps([
                "Loads glossary terms that must stay untranslated",
                "Adapts idioms, currency and date formats per locale",
                "Flags strings that expand past UI button limits",
            ])
            .with_needs(["Read brand glossary in Drive"]),
            AgentSkill::new(
                "slide-deck",
                "Slide deck",
                "Turns a brief into a short, clean deck",
                SkillCategory::Documents,
            )
            .with_visual(SkillGlyph::Slides, SkillTone::Purple)
            .with_source(SkillSource::Builtin, "v1.6")
            .with_state(false, false)
            .with_installs("10.3K use it")
            .with_triggers([
                "Turn this memo into an 8-slide deck",
                "Make a customer review presentation",
            ])
            .with_steps([
                "Outlines one clear headline per slide",
                "Pairs claims with charts or concise 3-bullet evidence",
                "Exports clean slides in workspace typography",
            ])
            .with_needs(["Create presentations in Google Drive"]),
            AgentSkill::new(
                "invoice-extraction",
                "Invoice extraction",
                "Pulls amounts, dates and vendors out of invoices",
                SkillCategory::Data,
            )
            .with_visual(SkillGlyph::Receipt, SkillTone::Amber)
            .with_source(SkillSource::Community("@jo".into()), "v1.4")
            .with_state(false, false)
            .with_installs("3.4K use it")
            .with_triggers([
                "Extract line items from October PDF invoices",
                "Log this receipt to the finance sheet",
            ])
            .with_steps([
                "Reads vendor, invoice number, tax and due date",
                "Checks totals against line-item sums",
                "Appends a verified row to the finance ledger",
            ])
            .with_needs(["Read Gmail attachments", "Edit finance sheet"]),
            AgentSkill::new(
                "interview-synthesis",
                "Interview synthesis",
                "Finds the patterns across customer interviews",
                SkillCategory::Research,
            )
            .with_visual(SkillGlyph::Users, SkillTone::Pink)
            .with_source(SkillSource::Community("@nora".into()), "v1.2")
            .with_state(false, false)
            .with_installs("2K use it")
            .with_triggers([
                "What themes came up across these 6 user calls?",
                "Group onboarding friction quotes by topic",
            ])
            .with_steps([
                "Tags recurring pains, workflows and feature requests",
                "Links every theme to verbatim customer quotes",
                "Ranks opportunities by frequency and urgency",
            ])
            .with_needs(["Read call transcripts in Drive"]),
            AgentSkill::new(
                "notes-to-tasks",
                "Notes to tasks",
                "Turns any notes into tasks with owners and dates",
                SkillCategory::Documents,
            )
            .with_visual(SkillGlyph::Notes, SkillTone::Blue)
            .with_source(SkillSource::Builtin, "v1.7")
            .with_state(false, false)
            .with_installs("7.6K use it")
            .with_triggers([
                "Turn these workshop notes into Linear tasks",
                "Pull follow-ups with owners from my scratchpad",
            ])
            .with_steps([
                "Finds commitments, owners and target dates",
                "Deduplicates against open Linear issues",
                "Creates draft tasks ready for one-click confirm",
            ])
            .with_needs(["Create issues in Linear"]),
        ]
    }

    /// Replace the full skills catalog.
    pub fn with_skills(mut self, skills: Vec<AgentSkill>) -> Self {
        if self
            .selected_id
            .as_ref()
            .is_none_or(|sel| !skills.iter().any(|s| &s.id == sel))
        {
            self.selected_id = skills.first().map(|s| s.id.clone());
        }
        self.skills = skills;
        self
    }

    /// Override the initially selected skill open in the right inspector.
    pub fn with_selected(mut self, selected: Option<impl Into<SharedString>>) -> Self {
        self.selected_id = selected.map(Into::into);
        self
    }

    /// Set the initial category filter tab.
    pub fn with_tab(mut self, tab: SkillFilterTab) -> Self {
        self.active_tab = tab;
        self
    }

    /// Configure skeleton loading placeholder mode.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure explicit empty state mode.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Render only the skills canvas without the outer [`AppShell`] chrome.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Update loading mode dynamically.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Update empty state mode dynamically.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the current skills slice.
    pub fn skills(&self) -> &[AgentSkill] {
        &self.skills
    }

    /// Return the currently selected skill id.
    pub fn selected_id(&self) -> Option<&SharedString> {
        self.selected_id.as_ref()
    }

    /// Select a skill and open the right-hand inspector.
    pub fn select_skill(&mut self, id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id = id.into();
        if self.selected_id.as_ref() != Some(&id) {
            self.show_skill_md = false;
        }
        self.selected_id = Some(id.clone());
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentSkillsEvent::Select(id));
        cx.notify();
    }

    /// Toggle an installed skill on or off.
    pub fn toggle_skill(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(sk) = self
            .skills
            .iter_mut()
            .find(|s| s.id.as_ref() == id && s.installed)
        {
            sk.enabled = !sk.enabled;
            let enabled = sk.enabled;
            let sid = sk.id.clone();
            cx.emit(AgentSkillsEvent::Toggle { id: sid, enabled });
            cx.notify();
        }
    }

    /// Install a Discover skill (with ring fill animation unless reduced motion is active).
    pub fn install_skill(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(sk) = self
            .skills
            .iter()
            .find(|s| s.id.as_ref() == id && !s.installed)
        else {
            return;
        };
        if self
            .installing
            .as_ref()
            .is_some_and(|anim| anim.skill_id.as_ref() == id)
        {
            return;
        }
        let sid = sk.id.clone();
        if Theme::global(cx).reduced_motion {
            self.finalize_install(sid.as_ref(), cx);
        } else {
            self.installing = Some(InstallAnim {
                skill_id: sid,
                started_at: Instant::now(),
            });
            cx.notify();
        }
    }

    fn finalize_install(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(sk) = self.skills.iter_mut().find(|s| s.id.as_ref() == id) {
            sk.installed = true;
            sk.enabled = true;
            let sid = sk.id.clone();
            self.fresh_landed = Some((sid.clone(), Instant::now()));
            cx.emit(AgentSkillsEvent::Install(sid));
            cx.notify();
        }
    }

    /// Remove a custom or community skill (built-in skills cannot be removed).
    pub fn remove_skill(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(pos) = self.skills.iter().position(|s| s.id.as_ref() == id) else {
            return;
        };
        if !self.skills[pos].source.is_removable() {
            return;
        }
        let sid = self.skills[pos].id.clone();
        match self.skills[pos].source {
            SkillSource::Community(_) => {
                self.skills[pos].installed = false;
                self.skills[pos].enabled = false;
            }
            SkillSource::Yours => {
                self.skills.remove(pos);
                if self.selected_id.as_ref() == Some(&sid) {
                    self.selected_id = self.skills.first().map(|s| s.id.clone());
                }
            }
            SkillSource::Builtin => return,
        }
        cx.emit(AgentSkillsEvent::Remove(sid));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search skills", "搜索技能").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |_this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                EditEvent::Escape => {
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.describe_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "e.g. Every time I paste customer feedback, tag the theme and link matching Linear issues…",
                "例如：每次粘贴客户反馈时，自动归类主题并关联对应的 Linear 问题…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed) {
                    cx.notify();
                }
            })
            .detach();
            self.describe_editor = Some(ed);
        }

        if self.name_editor.is_none() {
            let ph: SharedString = tr(cx, "Skill name", "技能名称").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted) {
                    cx.notify();
                }
            })
            .detach();
            self.name_editor = Some(ed);
        }

        if self.desc_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Short summary of what this skill does",
                "简要说明此技能的用途",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed) {
                    cx.notify();
                }
            })
            .detach();
            self.desc_editor = Some(ed);
        }
    }

    fn search_query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default()
    }

    fn sync_animations(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(anim) = self.installing.clone() {
            let elapsed_ms = anim.started_at.elapsed().as_secs_f32() * 1000.;
            if elapsed_ms >= INSTALL_RING_MS + INSTALL_DONE_HOLD_MS {
                self.installing = None;
                self.finalize_install(anim.skill_id.as_ref(), cx);
            }
            w.request_animation_frame();
        }

        if let Some((_, landed_at)) = self.fresh_landed {
            if landed_at.elapsed().as_secs_f32() * 1000. >= 1400. {
                self.fresh_landed = None;
            } else if !Theme::global(cx).reduced_motion {
                w.request_animation_frame();
            }
        }

        if self.modal_step == Some(NewSkillStep::Drafting)
            && let Some(started) = self.draft_started
        {
            if started.elapsed().as_secs_f32() * 1000. >= DRAFT_TWINKLE_MS {
                self.draft_started = None;
                self.populate_drafted_skill(w, cx);
                self.modal_step = Some(NewSkillStep::Form);
            }
            w.request_animation_frame();
        }
    }

    fn populate_drafted_skill(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let prompt_txt = self
            .describe_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        let lower = prompt_txt.to_ascii_lowercase();
        let (title, cat, desc, triggers, steps) = if lower.contains("contract") {
            (
                "Contract clause check",
                SkillCategory::Documents,
                "Flags non-standard liability, renewal and IP clauses before legal sign-off",
                vec![
                    SharedString::from("Check this MSA for non-standard clauses"),
                    SharedString::from("Review vendor DPA before signing"),
                ],
                vec![
                    SharedString::from(
                        "Compares liability cap and auto-renewal terms against playbook",
                    ),
                    SharedString::from("Quotes risky paragraphs with plain-language redlines"),
                ],
            )
        } else if lower.contains("stripe") || lower.contains("payout") {
            (
                "Payout reconciliation",
                SkillCategory::Data,
                "Reconciles daily Stripe payouts, fees and refunds into the finance sheet",
                vec![
                    SharedString::from("Summarize today's Stripe payout"),
                    SharedString::from("Why was net payout lower than gross?"),
                ],
                vec![
                    SharedString::from("Groups charges, refunds and processing fees by currency"),
                    SharedString::from("Flags disputed charges and updates ledger totals"),
                ],
            )
        } else {
            (
                "Support FAQ builder",
                SkillCategory::Writing,
                "Turns resolved support threads into clear, searchable help articles",
                vec![
                    SharedString::from("Turn this ticket thread into an FAQ entry"),
                    SharedString::from("Draft a help doc from recent questions"),
                ],
                vec![
                    SharedString::from("Strips customer-specific names and private tokens"),
                    SharedString::from("Writes a step-by-step answer in Fieldwork's brand voice"),
                ],
            )
        };

        self.form_category = cat;
        self.form_triggers = triggers;
        self.form_steps = steps;
        if let Some(ne) = &self.name_editor {
            ne.update(cx, |e, cx| e.set_value(title, w, cx));
        }
        if let Some(de) = &self.desc_editor {
            de.update(cx, |e, cx| e.set_value(desc, w, cx));
        }
    }

    fn commit_form_skill(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let raw_name = self
            .name_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if raw_name.is_empty() {
            return;
        }
        let raw_desc = self
            .desc_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Custom workflow authored in Fieldwork".to_string());

        let id_str = format!("custom-{}", self.skills.len() + 1);
        let triggers = if self.form_triggers.is_empty() {
            vec![SharedString::from(format!("Run {raw_name}"))]
        } else {
            self.form_triggers.clone()
        };
        let steps = if self.form_steps.is_empty() {
            vec![
                SharedString::from("Reads current conversation and workspace context"),
                SharedString::from(raw_desc.clone()),
            ]
        } else {
            self.form_steps.clone()
        };

        let skill = AgentSkill::new(id_str.clone(), raw_name, raw_desc, self.form_category)
            .with_visual(SkillGlyph::Sparkles, SkillTone::Blue)
            .with_source(SkillSource::Yours, "v1.0")
            .with_state(true, true)
            .with_triggers(triggers)
            .with_steps(steps)
            .with_needs(["Workspace files"]);

        self.skills.insert(0, skill.clone());
        self.selected_id = Some(id_str.clone().into());
        self.fresh_landed = Some((id_str.into(), Instant::now()));
        self.modal_step = None;
        self.empty = false;
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));

        if let Some(ne) = &self.name_editor {
            ne.update(cx, |e, cx| e.set_value("", w, cx));
        }
        if let Some(de) = &self.desc_editor {
            de.update(cx, |e, cx| e.set_value("", w, cx));
        }
        if let Some(pe) = &self.describe_editor {
            pe.update(cx, |e, cx| e.set_value("", w, cx));
        }
        self.form_triggers.clear();
        self.form_steps.clear();

        cx.emit(AgentSkillsEvent::Create(skill));
        cx.notify();
    }

    fn ordered_visible_indices(&self, cx: &App) -> Vec<usize> {
        let q = self.search_query(cx);
        let (mut inst, disc) = filter_skills_catalog(&self.skills, self.active_tab, &q);
        match self.scope_tab {
            SkillScopeTab::All => {
                inst.extend(disc);
                inst
            }
            SkillScopeTab::Installed => inst,
            SkillScopeTab::Discover => disc,
        }
    }

    fn step_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let order = self.ordered_visible_indices(cx);
        if order.is_empty() {
            return;
        }
        let cur = self
            .selected_id
            .as_ref()
            .and_then(|sel| order.iter().position(|&idx| &self.skills[idx].id == sel))
            .unwrap_or(0);
        let next = (cur as isize + delta).clamp(0, order.len() as isize - 1) as usize;
        let next_id = self.skills[order[next]].id.clone();
        if self.selected_id.as_ref() != Some(&next_id) {
            self.show_skill_md = false;
            self.selected_id = Some(next_id.clone());
            cx.emit(AgentSkillsEvent::Select(next_id));
            cx.notify();
        }
    }

    fn on_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if self.modal_step.is_some() {
            if key == "escape" {
                cx.stop_propagation();
                self.modal_step = None;
                self.draft_started = None;
                cx.notify();
            } else if key == "enter"
                && ev.keystroke.modifiers.platform
                && self.modal_step == Some(NewSkillStep::Form)
            {
                cx.stop_propagation();
                self.commit_form_skill(w, cx);
            }
            return;
        }

        if self
            .search_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus_handle(cx).is_focused(w))
        {
            if key == "down" {
                cx.stop_propagation();
                self.focus.focus(w, cx);
                self.step_selection(0, cx);
            }
            return;
        }

        let plain = !ev.keystroke.modifiers.platform
            && !ev.keystroke.modifiers.control
            && !ev.keystroke.modifiers.alt;
        if !plain {
            return;
        }

        match key {
            "/" => {
                if let Some(ed) = &self.search_editor {
                    cx.stop_propagation();
                    let fh = ed.read(cx).focus_handle(cx);
                    fh.focus(w, cx);
                }
            }
            "j" | "right" | "down" => {
                cx.stop_propagation();
                self.step_selection(1, cx);
            }
            "k" | "left" | "up" => {
                cx.stop_propagation();
                self.step_selection(-1, cx);
            }
            "enter" => {
                if let Some(sel) = self.selected_id.clone() {
                    cx.stop_propagation();
                    self.select_skill(sel, cx);
                }
            }
            " " | "space" => {
                if let Some(sel) = self.selected_id.clone() {
                    cx.stop_propagation();
                    self.toggle_skill(sel.as_ref(), cx);
                }
            }
            "escape" => {
                if !self.search_query(cx).is_empty() {
                    if let Some(ed) = &self.search_editor {
                        cx.stop_propagation();
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                        cx.notify();
                    }
                } else if self.shell.read(cx).is_panel_open() {
                    cx.stop_propagation();
                    self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                }
            }
            _ => {}
        }
    }

    fn render_header(&self, compact: bool, w: &Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();
        let query_non_empty = !self.search_query(cx).is_empty();
        let search_focused = self
            .search_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus_handle(cx).is_focused(w));

        let search_active = search_focused || query_non_empty;
        let search_box = div()
            .ml_auto()
            .w(px(if compact && !search_active {
                128.
            } else {
                220.
            }))
            .h(px(28.))
            .px(px(8.))
            .rounded(px(6.))
            .bg(if search_active { p.card } else { p.chip })
            .when(search_active, |d| d.border_1().border_color(p.line_strong))
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_icon(ProIcon::Search, 14., p.faint))
            .when_some(self.search_editor.clone(), |d, ed| {
                d.child(div().flex_1().min_w_0().child(ed))
            })
            .when(query_non_empty, |d| {
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "clear-search"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Clear search", "清空搜索"))
                        .tab_index(0)
                        .size(px(16.))
                        .rounded(px(4.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Close, 10., p.faint)),
                    cx,
                    |this, w, cx| {
                        if let Some(ed) = &this.search_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        cx.notify();
                    },
                ))
            });

        let new_btn = if compact {
            pro_icon_btn(
                pro_eid(&self.id, "new-skill-compact"),
                tr(cx, "New skill", "新建技能"),
                ProIcon::Plus,
                28.,
                false,
                cx,
                |this, _, cx| {
                    this.modal_step = Some(NewSkillStep::ChooseWay);
                    cx.notify();
                },
            )
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .into_any_element()
        } else {
            activate(
                div()
                    .id(pro_eid(&self.id, "new-skill"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "New skill", "新建技能"))
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.border_color(p.line_strong).bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Plus, 14., p.muted))
                    .child(tr(cx, "New skill", "新建技能")),
                cx,
                |this, _, cx| {
                    this.modal_step = Some(NewSkillStep::ChooseWay);
                    cx.notify();
                },
            )
            .into_any_element()
        };

        div()
            .h(px(48.))
            .w_full()
            .flex_none()
            .pl(px(if compact && !self.standalone {
                46.
            } else {
                20.
            }))
            .pr(px(if !panel_open && !self.standalone {
                52.
            } else {
                12.
            }))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(if compact { 8. } else { 12. }))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Skills", "技能")),
            )
            .child(search_box)
            .child(new_btn)
    }

    fn render_category_chips(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let cat_sensor = pro_selection_sensor();
        let cat_key: SharedString = self.active_tab.key().into();
        let mut tabs: Vec<(SkillFilterTab, SharedString)> =
            vec![(SkillFilterTab::All, tr(cx, "All", "全部").into())];
        for cat in SkillCategory::ALL {
            tabs.push((SkillFilterTab::Category(cat), cat.label(cx).into()));
        }

        let mut row = div()
            .id(pro_eid(&self.id, "cat-chips"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Categories", "技能分类"))
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(4.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "cat-pill-ind"),
                cat_key,
                cat_sensor.clone(),
                6.,
                p.chip,
                None,
                false,
            ));

        for (tab, label) in tabs {
            let selected = self.active_tab == tab;
            row = row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("cat-{label}")))
                    .role(Role::Tab)
                    .aria_label(label.clone())
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .hover(move |s| {
                        if selected {
                            s.text_color(p.ink)
                        } else {
                            s.text_color(p.ink).bg(p.hover)
                        }
                    })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.muted })
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .when(selected, |d| d.child(pro_mark_selected(&cat_sensor)))
                    .child(label),
                cx,
                move |this, _, cx| {
                    this.active_tab = tab;
                    cx.notify();
                },
            ));
        }

        let scope_sensor = pro_selection_sensor();
        let scope_key: SharedString = self.scope_tab.key().into();
        let scope_defs = [
            (SkillScopeTab::All, tr(cx, "All", "全部")),
            (SkillScopeTab::Installed, tr(cx, "Installed", "已安装")),
            (SkillScopeTab::Discover, tr(cx, "Discover", "发现")),
        ];
        let mut scope_strip = div()
            .id(pro_eid(&self.id, "scope-tabs"))
            .role(Role::TabList)
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "scope-pill-ind"),
                scope_key,
                scope_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));
        for (sc, slbl) in scope_defs {
            let sel = self.scope_tab == sc;
            scope_strip = scope_strip.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("scope-{}", sc.key())))
                    .role(Role::Tab)
                    .aria_label(slbl)
                    .tab_index(0)
                    .relative()
                    .h(px(24.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(if sel {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if sel { p.ink } else { p.muted })
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .when(sel, |d| d.child(pro_mark_selected(&scope_sensor)))
                    .child(slbl),
                cx,
                move |this, _, cx| {
                    this.scope_tab = sc;
                    cx.notify();
                },
            ));
        }

        div()
            .mb(px(-10.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(row)
            .child(scope_strip)
    }

    fn render_skill_card(&self, skill: &AgentSkill, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let selected = self.selected_id.as_ref() == Some(&skill.id);
        let is_off = skill.installed && !skill.enabled;
        let is_fresh = self
            .fresh_landed
            .as_ref()
            .is_some_and(|(fid, _)| fid == &skill.id);

        let sid_select = skill.id.clone();
        let sid_corner = skill.id.clone();

        let tone_c = skill.tone.resolve(&p);
        let mark_el = if is_off {
            div()
                .size(px(20.))
                .flex_none()
                .rounded(px(5.))
                .bg(p.chip)
                .flex()
                .items_center()
                .justify_center()
                .child(skill.glyph.render(12., p.faint))
        } else {
            div()
                .size(px(20.))
                .flex_none()
                .rounded(px(5.))
                .bg(tone_c.opacity(0.14))
                .flex()
                .items_center()
                .justify_center()
                .child(skill.glyph.render(12., tone_c))
        };

        let src_badge = skill.source.badge_label(cx);
        let aria_card = if skill.installed {
            if skill.enabled {
                format!(
                    "{}: {}. {}, {}× this week",
                    skill.name, skill.description, src_badge, skill.weekly_uses
                )
            } else {
                format!("{}: {}. {}, Off", skill.name, skill.description, src_badge)
            }
        } else {
            format!("{}: {}. {}", skill.name, skill.description, src_badge)
        };

        let mut foot = div()
            .mt(px(4.))
            .flex()
            .items_center()
            .gap(px(6.))
            .min_w_0()
            .child(
                div()
                    .h(px(18.))
                    .px(px(6.))
                    .rounded(px(5.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .truncate()
                    .child(src_badge),
            );

        for app in skill.apps.iter().take(2) {
            foot = foot.child(pro_app_tile(app, 16., cx));
        }

        if !skill.installed
            && let Some(ref inst_txt) = skill.community_installs
        {
            foot = foot.child(
                div()
                    .ml_auto()
                    .text_size(px(11.5))
                    .text_color(p.faint)
                    .whitespace_nowrap()
                    .child(inst_txt.clone()),
            );
        }

        let main_btn = activate(
            div()
                .id(pro_eid(&self.id, format!("card-{}", skill.id)))
                .role(Role::Button)
                .aria_label(aria_card)
                .tab_index(0)
                .flex_1()
                .min_w_0()
                .px(px(14.))
                .py(px(12.))
                .rounded(px(10.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .cursor_pointer()
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(
                    div()
                        .h(px(24.))
                        .pr(px(36.))
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(mark_el)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if is_off { p.faint } else { p.ink })
                                .child(skill.name.clone()),
                        ),
                )
                .child(
                    div()
                        .h(px(36.))
                        .overflow_hidden()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(if is_off { p.faint } else { p.muted })
                        .child(skill.description.clone()),
                )
                .child(foot),
            cx,
            move |this, _, cx| {
                this.select_skill(sid_select.clone(), cx);
            },
        );

        let corner_el = if skill.installed {
            let sw_aria = format!(
                "{}: {}",
                skill.name,
                if skill.enabled {
                    tr(cx, "Turn off", "关闭")
                } else {
                    tr(cx, "Turn on", "开启")
                }
            );
            skill_card_switch(
                pro_eid(&self.id, format!("sw-{}", skill.id)),
                sw_aria,
                skill.enabled,
                cx,
                move |this, _, cx| {
                    this.toggle_skill(sid_corner.as_ref(), cx);
                },
            )
            .into_any_element()
        } else {
            let install_state = self
                .installing
                .as_ref()
                .filter(|a| a.skill_id == skill.id)
                .map(|a| a.started_at.elapsed().as_secs_f32() * 1000.);

            match install_state {
                Some(ms) if ms < INSTALL_RING_MS => {
                    let pct = (ms / INSTALL_RING_MS).clamp(0.05, 1.);
                    div()
                        .size(px(24.))
                        .rounded(px(6.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_progress_ring(pct, 13., 1.8, p.blue, p.line_strong))
                        .into_any_element()
                }
                Some(_) => div()
                    .size(px(24.))
                    .rounded(px(6.))
                    .bg(p.accent_soft)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Check, 13., p.blue))
                    .into_any_element(),
                None => activate(
                    div()
                        .id(pro_eid(&self.id, format!("inst-{}", skill.id)))
                        .role(Role::Button)
                        .aria_label(format!("{} {}", tr(cx, "Install", "安装"), skill.name))
                        .tab_index(0)
                        .size(px(24.))
                        .rounded(px(6.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Plus, 13., p.muted)),
                    cx,
                    move |this, _, cx| {
                        this.install_skill(sid_corner.as_ref(), cx);
                    },
                )
                .into_any_element(),
            }
        };

        let border_c = if is_fresh {
            p.blue.opacity(0.55)
        } else if selected {
            p.line_strong
        } else {
            p.line
        };
        let card_hover: Hsla = if p.dark {
            rgb(0x202024).into()
        } else {
            rgb(0xfcfcfd).into()
        };

        div()
            .relative()
            .flex_1()
            .min_w_0()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(border_c)
            .hover(move |s| s.bg(card_hover).border_color(p.line_strong))
            .flex()
            .child(main_btn)
            .child(
                div()
                    .absolute()
                    .top(px(12.))
                    .right(px(12.))
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .child(corner_el),
            )
    }

    fn render_skill_grid(&self, indices: &[usize], cols: usize, cx: &mut Context<Self>) -> Div {
        let cols = cols.max(1);
        let mut grid = div().w_full().flex().flex_col().gap(px(10.));
        for chunk in indices.chunks(cols) {
            let mut row = div().w_full().flex().gap(px(10.));
            for &idx in chunk {
                let sk = self.skills[idx].clone();
                row = row.child(self.render_skill_card(&sk, cx));
            }
            for _ in chunk.len()..cols {
                row = row.child(div().flex_1().min_w_0());
            }
            grid = grid.child(row);
        }
        grid
    }

    fn render_loading_body(&self, cols: usize, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let ph = 0.22_f32;
        let mut grid = div().w_full().flex().flex_col().gap(px(10.));
        for r in 0..2usize {
            let mut row = div().w_full().flex().gap(px(10.));
            for c in 0..cols.max(1) {
                let phase = ph + (r * 3 + c) as f32 * 0.08;
                row = row.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .px(px(14.))
                        .py(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(pro_skeleton_bar(20., 20., 5., phase, cx))
                                        .child(pro_skeleton_bar(92., 12., 4., phase, cx)),
                                )
                                .child(pro_skeleton_bar(26., 16., 8., phase + 0.1, cx)),
                        )
                        .child(pro_skeleton_bar(168., 11., 4., phase + 0.12, cx))
                        .child(pro_skeleton_bar(124., 11., 4., phase + 0.16, cx))
                        .child(pro_skeleton_bar(48., 18., 5., phase + 0.2, cx)),
                );
            }
            grid = grid.child(row);
        }

        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_skeleton_bar(36., 24., 6., ph, cx))
                    .child(pro_skeleton_bar(56., 24., 6., ph + 0.08, cx))
                    .child(pro_skeleton_bar(66., 24., 6., ph + 0.16, cx))
                    .child(pro_skeleton_bar(76., 24., 6., ph + 0.24, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(pro_skeleton_bar(82., 13., 4., ph, cx))
                    .child(grid),
            )
    }

    fn render_new_skill_modal(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let step = self.modal_step?;
        let p = ProPalette::get(cx);
        let well_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.032)
        };

        let (title, subtitle, show_back) = match step {
            NewSkillStep::ChooseWay => (
                tr(cx, "New skill", "新建技能"),
                tr(
                    cx,
                    "Teach your agent a repeatable workflow",
                    "为智能体添加可复用的工作流技能",
                ),
                false,
            ),
            NewSkillStep::Describe | NewSkillStep::Drafting => (
                tr(cx, "Describe it", "自然语言描述"),
                tr(
                    cx,
                    "Tell the agent what to do – it drafts triggers and steps",
                    "说明希望完成的事项——智能体会自动起草触发词与步骤",
                ),
                true,
            ),
            NewSkillStep::Form => (
                tr(cx, "Write skill", "编辑技能"),
                tr(
                    cx,
                    "Name, category and instructions saved as SKILL.md",
                    "填写名称、分类与说明（以 SKILL.md 结构保存）",
                ),
                true,
            ),
            NewSkillStep::Upload => (
                tr(cx, "Upload a file", "上传文件"),
                tr(
                    cx,
                    "Import an existing SKILL.md file",
                    "导入现有的 SKILL.md 技能说明文件",
                ),
                true,
            ),
        };

        let modal_head = div()
            .pt(px(16.))
            .pb(px(6.))
            .pl(px(if show_back { 12. } else { 20. }))
            .pr(px(12.))
            .flex()
            .items_start()
            .gap(px(8.))
            .when(show_back, |d| {
                d.child(pro_icon_btn(
                    pro_eid(&self.id, "modal-back"),
                    tr(cx, "Back", "返回"),
                    ProIcon::ChevronLeft,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.modal_step = Some(NewSkillStep::ChooseWay);
                        this.draft_started = None;
                        cx.notify();
                    },
                ))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.muted)
                            .child(subtitle),
                    ),
            )
            .child(pro_icon_btn(
                pro_eid(&self.id, "modal-close"),
                tr(cx, "Close", "关闭"),
                ProIcon::Close,
                28.,
                false,
                cx,
                |this, _, cx| {
                    this.modal_step = None;
                    this.draft_started = None;
                    cx.notify();
                },
            ));

        let body_and_foot = match step {
            NewSkillStep::ChooseWay => {
                let ways: [(&str, ProIcon, &str, &str, NewSkillStep, bool); 4] = [
                    (
                        "describe",
                        ProIcon::Sparkles,
                        tr(cx, "Describe it", "描述生成"),
                        tr(
                            cx,
                            "Tell the agent how you work – it drafts triggers and steps",
                            "向智能体描述你的做法，自动生成触发语句与执行步骤",
                        ),
                        NewSkillStep::Describe,
                        false,
                    ),
                    (
                        "write",
                        ProIcon::Code,
                        tr(cx, "Write it yourself", "手动编写"),
                        tr(
                            cx,
                            "Fill in the name, category and workflow instructions",
                            "自行填写技能名称、分类与工作流步骤",
                        ),
                        NewSkillStep::Form,
                        false,
                    ),
                    (
                        "upload",
                        ProIcon::Upload,
                        tr(cx, "Upload a file", "上传 SKILL.md"),
                        tr(
                            cx,
                            "Read triggers and steps from a SKILL.md file",
                            "从 SKILL.md 文件读取触发词与步骤",
                        ),
                        NewSkillStep::Upload,
                        false,
                    ),
                    (
                        "browse",
                        ProIcon::Search,
                        tr(cx, "Browse skills", "浏览技能库"),
                        tr(
                            cx,
                            "Add ready-to-use community and built-in skills from Discover",
                            "从下方发现区直接添加社区与内置技能",
                        ),
                        NewSkillStep::ChooseWay,
                        true,
                    ),
                ];

                let mut list = div()
                    .px(px(12.))
                    .pt(px(10.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(4.));
                for (key, ic, way_title, way_sub, target_step, is_browse) in ways {
                    list = list.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("way-{key}")))
                            .role(Role::Button)
                            .aria_label(way_title)
                            .tab_index(0)
                            .p(px(10.))
                            .rounded(px(10.))
                            .hover(move |s| s.bg(well_bg))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .size(px(36.))
                                    .flex_none()
                                    .rounded(px(9.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(ic, 16., p.ink)),
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
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(way_title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(p.muted)
                                            .child(way_sub),
                                    ),
                            )
                            .child(pro_icon(ProIcon::ChevronRight, 14., p.faint)),
                        cx,
                        move |this, _, cx| {
                            if is_browse {
                                this.modal_step = None;
                                this.active_tab = SkillFilterTab::All;
                                this.scope_tab = SkillScopeTab::Discover;
                            } else {
                                this.modal_step = Some(target_step);
                            }
                            cx.notify();
                        },
                    ));
                }
                list
            }
            NewSkillStep::Describe => {
                let ideas = [
                    "Turn resolved support threads into FAQ articles",
                    "Check vendor contracts for non-standard clauses",
                    "Reconcile daily Stripe payouts into the ledger",
                ];
                let mut idea_row = div().flex().flex_wrap().gap(px(6.));
                for (ii, idea) in ideas.into_iter().enumerate() {
                    idea_row = idea_row.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("idea-{ii}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(p.line_strong)
                            .hover(move |s| s.bg(well_bg).text_color(p.ink))
                            .text_size(px(12.))
                            .text_color(p.text)
                            .cursor_pointer()
                            .child(idea),
                        cx,
                        move |this, w, cx| {
                            if let Some(ed) = &this.describe_editor {
                                ed.update(cx, |e, cx| e.set_value(idea, w, cx));
                            }
                            cx.notify();
                        },
                    ));
                }

                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px(px(20.))
                            .py(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(
                                div()
                                    .min_h(px(84.))
                                    .p(px(10.))
                                    .rounded(px(8.))
                                    .bg(well_bg)
                                    .border_1()
                                    .border_color(p.line)
                                    .when_some(self.describe_editor.clone(), |d, ed| d.child(ed)),
                            )
                            .child(idea_row),
                    )
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(pro_btn(
                                pro_eid(&self.id, "desc-cancel"),
                                tr(cx, "Cancel", "取消"),
                                None,
                                ProBtnStyle::Ghost,
                                false,
                                cx,
                                |this, _, cx| {
                                    this.modal_step = None;
                                    cx.notify();
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "desc-draft"),
                                tr(cx, "Draft skill", "生成草稿"),
                                Some(ProIcon::Sparkles),
                                ProBtnStyle::Primary,
                                false,
                                cx,
                                |this, w, cx| {
                                    if Theme::global(cx).reduced_motion {
                                        this.populate_drafted_skill(w, cx);
                                        this.modal_step = Some(NewSkillStep::Form);
                                    } else {
                                        this.draft_started = Some(Instant::now());
                                        this.modal_step = Some(NewSkillStep::Drafting);
                                    }
                                    cx.notify();
                                },
                            )),
                    )
            }
            NewSkillStep::Drafting => {
                let elapsed_ms = self
                    .draft_started
                    .map(|t| t.elapsed().as_secs_f32() * 1000.)
                    .unwrap_or(0.);
                div()
                    .py(px(40.))
                    .px(px(24.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.))
                    .child(pro_agent_avatar(
                        "skills-author",
                        p.blue,
                        34.,
                        true,
                        elapsed_ms,
                        cx,
                    ))
                    .child(div().text_size(px(13.)).text_color(p.muted).child(tr(
                        cx,
                        "Drafting triggers and steps…",
                        "正在起草触发词与执行步骤…",
                    )))
            }
            NewSkillStep::Upload => div()
                .px(px(20.))
                .pt(px(12.))
                .pb(px(20.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "upload-dropzone"))
                        .role(Role::Button)
                        .tab_index(0)
                        .py(px(30.))
                        .px(px(16.))
                        .rounded(px(12.))
                        .bg(well_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .hover(move |s| s.border_color(p.blue))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .child(
                            div()
                                .size(px(40.))
                                .mb(px(4.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::Upload, 18., p.ink)),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(tr(cx, "Import sample SKILL.md", "导入示例 SKILL.md")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child("Customer changelog · YAML frontmatter + steps"),
                        ),
                    cx,
                    |this, w, cx| {
                        if let Some(parsed) = parse_skill_markdown(SAMPLE_UPLOADED_SKILL_MD) {
                            this.form_category = parsed.category;
                            this.form_triggers = parsed.triggers;
                            this.form_steps = parsed.steps;
                            if let Some(ne) = &this.name_editor {
                                ne.update(cx, |e, cx| e.set_value(parsed.name.as_ref(), w, cx));
                            }
                            if let Some(de) = &this.desc_editor {
                                de.update(cx, |e, cx| {
                                    e.set_value(parsed.description.as_ref(), w, cx)
                                });
                            }
                            this.modal_step = Some(NewSkillStep::Form);
                            cx.notify();
                        }
                    },
                )),
            NewSkillStep::Form => {
                let form_cat_sensor = pro_selection_sensor();
                let form_cat_key: SharedString =
                    SkillFilterTab::Category(self.form_category).key().into();
                let mut cat_chips =
                    div()
                        .relative()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.))
                        .child(pro_pill_indicator(
                            pro_eid(&self.id, "form-cat-ind"),
                            form_cat_key,
                            form_cat_sensor.clone(),
                            6.,
                            p.chip,
                            None,
                            false,
                        ));
                for cat in SkillCategory::ALL {
                    let on = self.form_category == cat;
                    let lbl = cat.label(cx);
                    cat_chips = cat_chips.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("form-cat-{lbl}")))
                            .role(Role::RadioButton)
                            .tab_index(0)
                            .relative()
                            .h(px(26.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .hover(move |s| if on { s } else { s.bg(p.hover) })
                            .flex()
                            .items_center()
                            .text_size(px(12.5))
                            .font_weight(if on {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if on { p.ink } else { p.muted })
                            .cursor_pointer()
                            .when(on, |d| d.child(pro_mark_selected(&form_cat_sensor)))
                            .child(lbl),
                        cx,
                        move |this, _, cx| {
                            this.form_category = cat;
                            cx.notify();
                        },
                    ));
                }

                let has_name = self
                    .name_editor
                    .as_ref()
                    .is_some_and(|e| !e.read(cx).value().trim().is_empty());

                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px(px(20.))
                            .py(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(14.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "Name", "名称")),
                                    )
                                    .child(
                                        div()
                                            .h(px(34.))
                                            .px(px(10.))
                                            .rounded(px(8.))
                                            .bg(well_bg)
                                            .border_1()
                                            .border_color(p.line)
                                            .flex()
                                            .items_center()
                                            .when_some(self.name_editor.clone(), |d, ed| {
                                                d.child(div().flex_1().min_w_0().child(ed))
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "Category", "分类")),
                                    )
                                    .child(cat_chips),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "What it does", "说明")),
                                    )
                                    .child(
                                        div()
                                            .min_h(px(64.))
                                            .p(px(10.))
                                            .rounded(px(8.))
                                            .bg(well_bg)
                                            .border_1()
                                            .border_color(p.line)
                                            .when_some(self.desc_editor.clone(), |d, ed| {
                                                d.child(ed)
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(pro_btn(
                                pro_eid(&self.id, "form-cancel"),
                                tr(cx, "Cancel", "取消"),
                                None,
                                ProBtnStyle::Ghost,
                                false,
                                cx,
                                |this, _, cx| {
                                    this.modal_step = None;
                                    cx.notify();
                                },
                            ))
                            .child(
                                pro_btn(
                                    pro_eid(&self.id, "form-create"),
                                    tr(cx, "Add skill", "添加技能"),
                                    None,
                                    ProBtnStyle::Primary,
                                    false,
                                    cx,
                                    |this, w, cx| {
                                        this.commit_form_skill(w, cx);
                                    },
                                )
                                .when(!has_name, |d| d.opacity(0.45)),
                            ),
                    )
            }
        };

        let dialog_card = div()
            .w_full()
            .max_w(px(460.))
            .rounded(px(16.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(modal_head)
            .child(pro_step_transition(
                pro_eid(&self.id, "new-skill-step-vtx"),
                step.step_index(),
                body_and_foot,
            ));

        Some(
            div()
                .id(pro_eid(&self.id, "modal-scrim"))
                .absolute()
                .inset_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.modal_step = None;
                        this.draft_started = None;
                        cx.notify();
                    }),
                )
                .child(pro_modal_appear(
                    pro_eid(&self.id, "new-skill-appear"),
                    "open",
                    if p.dark { 0.42 } else { 0.16 },
                    dialog_card,
                )),
        )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        self.sync_animations(w, cx);

        let p = ProPalette::get(cx);
        let measure_cell = self.measured_width.clone();
        let page_w = measure_cell.get();
        let compact = page_w <= 560.;
        let body_w = (page_w - if compact { 24. } else { 32. }).min(720.);
        let cols = if compact || body_w < 448. {
            1
        } else if body_w < 680. {
            2
        } else {
            3
        };

        let header = self.render_header(compact, w, cx);

        let body_el = if self.loading {
            self.render_loading_body(cols, cx).into_any_element()
        } else if self.empty || self.skills.is_empty() {
            pro_empty_state(
                ProIcon::Sparkles,
                tr(cx, "No skills yet", "暂无技能"),
                tr(
                    cx,
                    "Add a skill from the catalog or describe how your team works so your agent can step in.",
                    "从目录添加技能，或描述团队的工作方式，让智能体自动代劳。",
                ),
                Some((
                    pro_eid(&self.id, "empty-new-skill"),
                    tr(cx, "New skill", "新建技能").into(),
                    Box::new(|this: &mut Self, _, cx| {
                        this.modal_step = Some(NewSkillStep::ChooseWay);
                        cx.notify();
                    }),
                )),
                cx,
            )
            .into_any_element()
        } else {
            let query = self.search_query(cx);
            let (raw_installed, raw_discover) =
                filter_skills_catalog(&self.skills, self.active_tab, &query);
            let show_installed = matches!(
                self.scope_tab,
                SkillScopeTab::All | SkillScopeTab::Installed
            );
            let show_discover =
                matches!(self.scope_tab, SkillScopeTab::All | SkillScopeTab::Discover);
            let installed_idx = if show_installed {
                raw_installed
            } else {
                Vec::new()
            };
            let discover_idx = if show_discover {
                raw_discover
            } else {
                Vec::new()
            };

            let mut grid_sections = div().flex().flex_col().gap(px(28.));

            if installed_idx.is_empty() && discover_idx.is_empty() {
                grid_sections = grid_sections.child(pro_empty_state(
                    ProIcon::Search,
                    tr(cx, "No matching skills", "未找到匹配的技能"),
                    tr(
                        cx,
                        "Try another search phrase or switch back to All categories.",
                        "请可改其他搜索词，或切回全部分类。",
                    ),
                    Some((
                        pro_eid(&self.id, "reset-filters"),
                        tr(cx, "Show all skills", "显示全部技能").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.active_tab = SkillFilterTab::All;
                            this.scope_tab = SkillScopeTab::All;
                            if let Some(ed) = &this.search_editor {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ));
            } else {
                if !installed_idx.is_empty() {
                    grid_sections = grid_sections.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(tr(cx, "Installed", "已安装")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(installed_idx.len().to_string()),
                                    ),
                            )
                            .child(self.render_skill_grid(&installed_idx, cols, cx)),
                    );
                }

                if !discover_idx.is_empty() {
                    grid_sections = grid_sections.child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(tr(cx, "Discover", "发现更多")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(discover_idx.len().to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .mb(px(12.))
                                    .text_size(px(12.5))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Skills others use – add one and your agent can do it right away",
                                        "其他人正在使用的技能——添加后智能体即可立即掌握",
                                    )),
                            )
                            .child(self.render_skill_grid(&discover_idx, cols, cx)),
                    );
                }
            }

            let grid_key = format!("{}-{}", self.active_tab.key(), self.scope_tab.key());
            div()
                .flex()
                .flex_col()
                .gap(px(28.))
                .child(self.render_category_chips(cx))
                .child(pro_view_transition(
                    pro_eid(&self.id, "skills-grid-vtx"),
                    grid_key,
                    grid_sections,
                ))
                .into_any_element()
        };

        let modal_el = self.render_new_skill_modal(cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
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
            .child(header)
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div().w_full().flex().justify_center().child(
                            div()
                                .w_full()
                                .max_w(px(752.))
                                .px(px(if compact { 12. } else { 16. }))
                                .pt(px(if compact { 12. } else { 20. }))
                                .pb(px(if compact { 32. } else { 40. }))
                                .child(body_el),
                        ),
                    ),
            )
            .when_some(modal_el, |d, m| d.child(m))
    }

    fn render_runs_chart(&self, skill: &AgentSkill, w: &mut Window, cx: &mut App) -> Div {
        let p = ProPalette::get(cx);
        let tone_c = skill.tone.resolve(&p);
        let max_val = skill.daily_runs.iter().copied().max().unwrap_or(0).max(6);
        let top_tick = if max_val <= 6 {
            6
        } else if max_val <= 10 {
            10
        } else {
            max_val.div_ceil(4) * 4
        };
        let mid_tick = top_tick / 2;
        let plot_h = 72.0_f32;

        let mut bars_row = div()
            .absolute()
            .inset_0()
            .px(px(4.))
            .flex()
            .items_end()
            .justify_between();

        for (di, &count) in skill.daily_runs.iter().enumerate() {
            let frac = (count as f32 / top_tick as f32).clamp(0., 1.);
            let target_h = if !skill.enabled || count == 0 {
                2.0
            } else {
                (frac * plot_h).max(4.0)
            };
            let bar_h = pro_spring_f32(
                pro_eid(&self.id, format!("skill-run-bar-{di}")),
                target_h,
                0.28,
                w,
                cx,
            )
            .clamp(2.0, plot_h);
            let bar_color = if !skill.enabled {
                p.ink.opacity(0.12)
            } else if di >= 7 {
                tone_c.opacity(0.74)
            } else {
                tone_c.opacity(0.22)
            };
            bars_row = bars_row.child(
                div()
                    .w(px(9.))
                    .h(px(bar_h))
                    .flex_none()
                    .rounded_t(px(3.))
                    .rounded_b(px(1.))
                    .bg(bar_color),
            );
        }

        let runs_right = if !skill.enabled {
            tr(cx, "Off", "已关闭").to_string()
        } else {
            format!("{}× {}", skill.weekly_uses, tr(cx, "this week", "本周"))
        };

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Runs", "运行频次")),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(runs_right),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .h(px(plot_h))
                                    .w(px(16.))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .items_end()
                                    .text_size(px(11.))
                                    .text_color(p.faint)
                                    .child(top_tick.to_string())
                                    .child(mid_tick.to_string())
                                    .child("0"),
                            )
                            .child(
                                div()
                                    .relative()
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(plot_h))
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .left_0()
                                            .right_0()
                                            .h(px(1.))
                                            .bg(p.line_soft),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(plot_h * 0.5))
                                            .left_0()
                                            .right_0()
                                            .h(px(1.))
                                            .bg(p.line_soft),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .bottom_0()
                                            .left_0()
                                            .right_0()
                                            .h(px(1.))
                                            .bg(p.line),
                                    )
                                    .child(bars_row),
                            ),
                    )
                    .child(
                        div()
                            .pl(px(26.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(tr(cx, "2 weeks ago", "两周前"))
                            .child(tr(cx, "Today", "今天")),
                    ),
            )
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);

        if self.loading {
            let ph = 0.2_f32;
            let skel = div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(22.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(40., 40., 9., ph, cx))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(pro_skeleton_bar(108., 15., 4., ph, cx))
                                .child(pro_skeleton_bar(72., 11., 4., ph + 0.1, cx)),
                        ),
                )
                .child(pro_skeleton_bar(230., 14., 4., ph + 0.1, cx))
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(84., 28., 6., ph + 0.15, cx))
                        .child(pro_skeleton_bar(76., 28., 6., ph + 0.2, cx)),
                )
                .child(pro_skeleton_bar(240., 88., 8., ph + 0.25, cx))
                .child(pro_skeleton_bar(240., 96., 10., ph + 0.3, cx))
                .into_any_element();
            return Some((tr(cx, "Skill", "技能").into(), ProIcon::Sparkles, skel));
        }

        if self.empty || self.skills.is_empty() {
            let empty_panel = div()
                .min_h(px(240.))
                .p(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.faint)
                .child(tr(cx, "Nothing open", "未打开任何技能"))
                .into_any_element();
            return Some((
                tr(cx, "Skill", "技能").into(),
                ProIcon::Sparkles,
                empty_panel,
            ));
        }

        let sel_id = self.selected_id.as_ref()?;
        let skill = self.skills.iter().find(|s| &s.id == sel_id)?.clone();
        let tone_c = skill.tone.resolve(&p);
        let well_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.038)
        } else {
            hsla(0., 0., 0., 0.032)
        };

        let sid_toggle = skill.id.clone();
        let sid_remove = skill.id.clone();
        let sid_install = skill.id.clone();

        let actions_row = if skill.installed {
            let toggle_txt = if skill.enabled {
                tr(cx, "Turn off", "关闭")
            } else {
                tr(cx, "Turn on", "开启")
            };
            let mut row = div()
                .mt(px(-10.))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "panel-toggle-skill"))
                        .role(Role::Button)
                        .aria_label(toggle_txt)
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(div().size(px(7.)).rounded_full().bg(if skill.enabled {
                            p.blue
                        } else {
                            p.faint
                        }))
                        .child(toggle_txt),
                    cx,
                    move |this, _, cx| {
                        this.toggle_skill(sid_toggle.as_ref(), cx);
                    },
                ));

            if skill.source.is_removable() {
                row = row.child(activate(
                    div()
                        .id(pro_eid(&self.id, "panel-remove-skill"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Remove", "移除"))
                        .tab_index(0)
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(10.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.chip).text_color(p.ink))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Trash, 14., p.muted))
                        .child(tr(cx, "Remove", "移除")),
                    cx,
                    move |this, _, cx| {
                        this.remove_skill(sid_remove.as_ref(), cx);
                    },
                ));
            }
            row
        } else {
            let install_ms = self
                .installing
                .as_ref()
                .filter(|a| a.skill_id == skill.id)
                .map(|a| a.started_at.elapsed().as_secs_f32() * 1000.);

            let inst_btn = match install_ms {
                Some(ms) if ms < INSTALL_RING_MS => {
                    let pct = (ms / INSTALL_RING_MS).clamp(0.05, 1.);
                    div()
                        .min_w(px(116.))
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.blue)
                        .child(pro_progress_ring(pct, 13., 1.8, p.blue, p.line_strong))
                        .child(tr(cx, "Installing…", "安装中…"))
                        .into_any_element()
                }
                Some(_) => div()
                    .min_w(px(104.))
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .bg(p.accent_soft)
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.blue)
                    .child(pro_icon(ProIcon::Check, 13., p.blue))
                    .child(tr(cx, "Installed", "已安装"))
                    .into_any_element(),
                None => activate(
                    div()
                        .id(pro_eid(&self.id, "panel-install-skill"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Install", "安装"))
                        .tab_index(0)
                        .min_w(px(96.))
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(10.))
                        .rounded(px(6.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Plus, 13., p.muted))
                        .child(tr(cx, "Install", "安装")),
                    cx,
                    move |this, _, cx| {
                        this.install_skill(sid_install.as_ref(), cx);
                    },
                )
                .into_any_element(),
            };

            let mut row = div()
                .mt(px(-10.))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.))
                .child(inst_btn);
            if let Some(ref uses) = skill.community_installs {
                row = row.child(
                    div()
                        .ml_auto()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(uses.clone()),
                );
            }
            row
        };

        let mut panel = div()
            .p(px(14.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .size(px(40.))
                            .flex_none()
                            .rounded(px(8.))
                            .bg(if skill.installed && !skill.enabled {
                                p.chip
                            } else {
                                tone_c.opacity(0.14)
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(skill.glyph.render(
                                22.,
                                if skill.installed && !skill.enabled {
                                    p.faint
                                } else {
                                    tone_c
                                },
                            )),
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
                                    .truncate()
                                    .text_size(px(15.))
                                    .line_height(px(22.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(skill.name.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .child(format!(
                                        "{} · {}",
                                        skill.source.badge_label(cx),
                                        skill.version
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .mt(px(-10.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(skill.description.clone()),
            )
            .child(actions_row);

        if skill.installed {
            panel = panel.child(self.render_runs_chart(&skill, w, cx));
        }

        if !skill.triggers.is_empty() {
            let mut trig_box = div()
                .rounded(px(10.))
                .bg(well_bg)
                .px(px(14.))
                .py(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.));
            for trig in &skill.triggers {
                trig_box = trig_box.child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(format!("“{trig}”")),
                );
            }
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Steps in when you say", "触发语句示例")),
                    )
                    .child(trig_box),
            );
        }

        if !skill.steps.is_empty() {
            let what_body = if self.show_skill_md {
                div()
                    .rounded(px(10.))
                    .bg(well_bg)
                    .px(px(14.))
                    .py(px(12.))
                    .font_family("Fira Code")
                    .text_size(px(11.5))
                    .line_height(px(18.))
                    .text_color(p.ink)
                    .child(skill.to_skill_markdown())
                    .into_any_element()
            } else {
                let mut steps_box = div()
                    .rounded(px(10.))
                    .bg(well_bg)
                    .px(px(14.))
                    .py(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(8.));
                for (si, st) in skill.steps.iter().enumerate() {
                    steps_box = steps_box.child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(10.))
                            .child(
                                div()
                                    .mt(px(1.))
                                    .size(px(18.))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child((si + 1).to_string()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.text)
                                    .child(st.clone()),
                            ),
                    );
                }
                steps_box.into_any_element()
            };

            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "What it does", "执行步骤")),
                            )
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "toggle-skill-md"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(20.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .when(self.show_skill_md, |d| d.bg(p.chip))
                                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.))
                                    .text_color(if self.show_skill_md { p.ink } else { p.faint })
                                    .cursor_pointer()
                                    .child("SKILL.md"),
                                cx,
                                |this, _, cx| {
                                    this.show_skill_md = !this.show_skill_md;
                                    cx.notify();
                                },
                            )),
                    )
                    .child(pro_view_transition(
                        pro_eid(&self.id, "skill-md-vtx"),
                        if self.show_skill_md { "md" } else { "steps" },
                        what_body,
                    )),
            );
        }

        if !skill.needs.is_empty() {
            let mut needs_wrap = div().flex().flex_wrap().items_center().gap(px(6.));
            for need in &skill.needs {
                needs_wrap = needs_wrap.child(
                    div()
                        .px(px(8.))
                        .py(px(3.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .text_color(p.text)
                        .child(pro_icon(ProIcon::Shield, 12., p.muted))
                        .child(need.clone()),
                );
            }
            needs_wrap = needs_wrap.child(activate(
                div()
                    .id(pro_eid(&self.id, "panel-permissions-link"))
                    .role(Role::Button)
                    .tab_index(0)
                    .ml_auto()
                    .h(px(24.))
                    .px(px(6.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child(tr(cx, "Permissions", "权限设置"))
                    .child(pro_icon(ProIcon::ChevronRight, 12., p.muted)),
                cx,
                |this, _, cx| {
                    this.shell.update(cx, |s, _| s.set_active("permissions"));
                    cx.emit(AgentSkillsEvent::Navigate("Permissions".into()));
                    cx.notify();
                },
            ));

            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "What it needs", "所需权限")),
                    )
                    .child(needs_wrap),
            );
        }

        if !skill.used_in.is_empty() {
            let mut used_col = div().flex().flex_col().gap(px(8.));
            for rec in &skill.used_in {
                used_col = used_col.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .text_size(px(13.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(p.text)
                                .child(rec.task.clone()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(rec.at.clone()),
                        ),
                );
            }
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Ran lately", "最近调用")),
                    )
                    .child(used_col),
            );
        }

        Some((
            skill.name.clone(),
            skill.glyph.tab_icon(),
            pro_view_transition(pro_eid(&self.id, "skill-insp-vtx"), skill.id.clone(), panel),
        ))
    }
}

impl Render for AgentSkillsScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[derive(IntoElement)]
struct SkillSwitchTrack {
    state_id: ElementId,
    checked: bool,
}

impl RenderOnce for SkillSwitchTrack {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let t = pro_spring_f32(
            self.state_id,
            if self.checked { 1.0 } else { 0.0 },
            0.24,
            w,
            cx,
        )
        .clamp(0., 1.);
        let off_bg: Hsla = if p.dark {
            rgb(0x3a3a3f).into()
        } else {
            rgb(0xd9d9dc).into()
        };
        let track_bg = pro_mix_color(off_bg, p.blue, t);
        div()
            .relative()
            .w(px(26.))
            .h(px(16.))
            .rounded_full()
            .bg(track_bg)
            .child(
                div()
                    .absolute()
                    .top(px(2.))
                    .left(px(2. + 10. * t))
                    .size(px(12.))
                    .rounded_full()
                    .bg(rgb(0xffffff)),
            )
    }
}

fn skill_card_switch<V: 'static>(
    id: impl Into<ElementId>,
    aria: impl Into<SharedString>,
    checked: bool,
    cx: &Context<V>,
    on_toggle: impl Fn(&mut V, &mut Window, &mut Context<V>) + Clone + 'static,
) -> Stateful<Div> {
    let p = ProPalette::get(cx);
    let id = id.into();
    let state_id = pro_eid(&id, "sw-spring");
    let el = div()
        .id(id)
        .role(Role::Switch)
        .aria_label(aria.into())
        .tab_index(0)
        .w(px(26.))
        .h(px(16.))
        .flex_none()
        .rounded_full()
        .cursor_pointer()
        .focus_visible(move |s| s.border_1().border_color(p.focus))
        .child(SkillSwitchTrack { state_id, checked });
    activate(el, cx, on_toggle)
}

#[cfg(test)]
mod tests {
    use super::{
        AgentSkillsScreen, SAMPLE_UPLOADED_SKILL_MD, SkillCategory, SkillFilterTab, SkillSource,
        filter_skills_catalog, parse_skill_markdown,
    };

    #[test]
    fn default_catalog_partitions_installed_and_discover_by_category_and_query() {
        let skills = AgentSkillsScreen::default_skills();
        let (inst_all, disc_all) = filter_skills_catalog(&skills, SkillFilterTab::All, "");
        assert_eq!(inst_all.len(), 9);
        assert_eq!(disc_all.len(), 6);

        let (inst_writing, disc_writing) = filter_skills_catalog(
            &skills,
            SkillFilterTab::Category(SkillCategory::Writing),
            "",
        );
        assert_eq!(inst_writing.len(), 2);
        assert_eq!(disc_writing.len(), 1);

        let (inst_q, disc_q) = filter_skills_catalog(&skills, SkillFilterTab::All, "invoice");
        assert_eq!(inst_q.len(), 0);
        assert_eq!(disc_q.len(), 1);
        assert_eq!(skills[disc_q[0]].id.as_ref(), "invoice-extraction");

        assert!(skills[0].source.is_removable());
        assert!(!SkillSource::Builtin.is_removable());
    }

    #[test]
    fn skill_markdown_roundtrip_parses_frontmatter_triggers_and_steps() {
        let skill = parse_skill_markdown(SAMPLE_UPLOADED_SKILL_MD)
            .expect("valid sample SKILL.md parses into AgentSkill");
        assert_eq!(skill.name.as_ref(), "Customer changelog");
        assert_eq!(skill.category, SkillCategory::Writing);
        assert_eq!(skill.triggers.len(), 2);
        assert_eq!(skill.steps.len(), 3);

        let exported = skill.to_skill_markdown();
        assert!(exported.contains("name: Customer changelog"));
        assert!(exported.contains("## Triggers"));
        assert!(exported.contains("## Steps"));
    }
}
