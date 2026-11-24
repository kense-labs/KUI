//! Feature-section agent gallery with slim gradient panels flanking an open product screen.
//!
//! Displays a centered carousel of slim vertical gradient panels flanking one wide open
//! agent workspace screen; clicking a side panel or pressing the arrow keys smoothly glides
//! that panel open while collapsing the previously open panel.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_eid, pro_empty_state, pro_mark_selected, pro_mix_color,
        pro_person_avatar, pro_pill_indicator, pro_selection_sensor, pro_skeleton_bar,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const CHEVRON_UP_DOWN_SVG: &str = r#"<path d="m8 10 4-4 4 4M8 14l4 4 4-4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#;

/// Compact KPI metric and sparkline series displayed inside an open [`GalleryItem`] screen.
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryKpi {
    /// Uppercase metric label (for example `"RUNS"` or `"SUCCESS"`).
    pub label: SharedString,
    /// Formatted tabular value (for example `"183"` or `"97%"`).
    pub value: SharedString,
    /// Normalized sparkline points (`0.0..=1.0`) rendered beside the metric.
    pub spark: Vec<f32>,
}

impl GalleryKpi {
    /// Create one KPI cell with `label`, `value`, and sparkline series.
    pub fn new(
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
        spark: Vec<f32>,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            spark,
        }
    }
}

/// Active tab inside the miniature agent workspace screen of [`AgentGalleryBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GalleryScreenTab {
    /// Recent automated execution runs and workflow steps.
    Runs,
    /// Connected third-party tools and skills granted to the agent.
    Tools,
    /// Goals, files, and rules the agent uses on every run.
    #[default]
    Context,
    /// Trigger condition, owner, and safety guardrail configuration.
    Settings,
}

impl GalleryScreenTab {
    const ALL: [Self; 4] = [Self::Runs, Self::Tools, Self::Context, Self::Settings];

    fn key(self) -> &'static str {
        match self {
            Self::Runs => "runs",
            Self::Tools => "tools",
            Self::Context => "context",
            Self::Settings => "settings",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Runs => tr(cx, "Runs", "运行"),
            Self::Tools => tr(cx, "Tools", "工具"),
            Self::Context => tr(cx, "Context", "上下文"),
            Self::Settings => tr(cx, "Settings", "设置"),
        }
    }
}

/// One agent panel specification in [`AgentGalleryBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryItem {
    /// Stable identifier (for example `"churn-watch"`).
    pub id: SharedString,
    /// Agent display title shown on the slim panel, topbar switcher, and hero heading.
    pub name: SharedString,
    /// One-line summary announced in the caption beneath the open panel.
    pub description: SharedString,
    /// Two-stop `[strong, soft]` artwork colors driving panel washes and agent marks.
    pub colors: [Hsla; 2],
    /// Event or schedule trigger phrase shown beneath the screen heading.
    pub trigger: SharedString,
    /// Human owner responsible for approvals and escalations.
    pub owner: SharedString,
    /// Relative last-run timestamp shown in the screen hero row.
    pub synced_ago: SharedString,
    /// Four KPI cells displayed across the open agent screen.
    pub kpis: [GalleryKpi; 4],
    /// Ordered playbook steps executed on each run.
    pub steps: Vec<SharedString>,
    /// Connected integration names used by the agent.
    pub skills: Vec<SharedString>,
    /// Safety guardrail rule enforced on every run.
    pub guardrail: SharedString,
}

impl GalleryItem {
    /// Construct a [`GalleryItem`] with `id`, `name`, `description`, and `[strong, soft]` colors.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
        strong: Hsla,
        soft: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            colors: [strong, soft],
            trigger: "Scheduled playbook trigger".into(),
            owner: "Lena Hart".into(),
            synced_ago: "3m ago".into(),
            kpis: [
                GalleryKpi::new(
                    "RUNS",
                    "183",
                    vec![0.35, 0.58, 0.22, 0.44, 0.68, 0.88, 0.40, 0.64, 0.85, 0.34],
                ),
                GalleryKpi::new(
                    "SUCCESS",
                    "97%",
                    vec![0.18, 0.88, 0.76, 0.65, 0.54, 0.42, 0.30, 0.18, 0.82, 0.70],
                ),
                GalleryKpi::new(
                    "TIME SAVED",
                    "3.3h",
                    vec![0.24, 0.62, 0.20, 0.56, 0.14, 0.52, 0.88, 0.44, 0.82, 0.38],
                ),
                GalleryKpi::new(
                    "ESCALATED",
                    "7",
                    vec![0.20, 0.28, 0.34, 0.42, 0.50, 0.58, 0.66, 0.74, 0.82, 0.88],
                ),
            ],
            steps: Vec::new(),
            skills: Vec::new(),
            guardrail: "Require human approval before external customer commits".into(),
        }
    }

    /// Override the `[strong, soft]` gradient pair for this panel.
    pub fn with_colors(mut self, strong: Hsla, soft: Hsla) -> Self {
        self.colors = [strong, soft];
        self
    }

    /// Set the trigger label displayed under the agent heading.
    pub fn with_trigger(mut self, trigger: impl Into<SharedString>) -> Self {
        self.trigger = trigger.into();
        self
    }

    /// Set the human owner displayed under the agent heading.
    pub fn with_owner(mut self, owner: impl Into<SharedString>) -> Self {
        self.owner = owner.into();
        self
    }

    /// Set the relative sync timestamp in the product screen hero row.
    pub fn with_synced_ago(mut self, synced_ago: impl Into<SharedString>) -> Self {
        self.synced_ago = synced_ago.into();
        self
    }

    /// Override the four KPI cells shown on the open agent screen.
    pub fn with_kpis(mut self, kpis: [GalleryKpi; 4]) -> Self {
        self.kpis = kpis;
        self
    }

    /// Attach ordered playbook steps displayed in the `Runs` tab and context feed.
    pub fn with_steps(mut self, steps: Vec<SharedString>) -> Self {
        self.steps = steps;
        self
    }

    /// Attach connected integration names displayed in the `Tools` tab.
    pub fn with_skills(mut self, skills: Vec<SharedString>) -> Self {
        self.skills = skills;
        self
    }

    /// Set the guardrail statement enforced on every run.
    pub fn with_guardrail(mut self, guardrail: impl Into<SharedString>) -> Self {
        self.guardrail = guardrail.into();
        self
    }
}

/// Typed events emitted by [`AgentGalleryBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentGalleryEvent {
    /// User opened a gallery panel by index and item id.
    Selected { index: usize, id: SharedString },
    /// User switched the miniature product screen tab on the active agent.
    TabChanged {
        item_id: SharedString,
        tab: GalleryScreenTab,
    },
    /// User added a context note to the active agent from the inline composer.
    ContextAdded {
        item_id: SharedString,
        text: SharedString,
    },
    /// User clicked `+ New agent` in the showcase screen header.
    NewAgentRequested,
}

fn default_gallery_items() -> Vec<GalleryItem> {
    vec![
        GalleryItem::new(
            "support-triage",
            "Support triage",
            "Classifies incoming tickets, drafts grounded replies and routes urgent bugs.",
            rgb(0xd96b68).into(),
            rgb(0xf3c1bf).into(),
        )
        .with_trigger("New ticket in Zendesk")
        .with_owner("Maya Lin")
        .with_synced_ago("1m ago")
        .with_kpis([
            GalleryKpi::new(
                "RUNS",
                "412",
                vec![0.22, 0.34, 0.30, 0.48, 0.56, 0.64, 0.72, 0.78, 0.86, 0.92],
            ),
            GalleryKpi::new(
                "SUCCESS",
                "99%",
                vec![0.72, 0.78, 0.76, 0.84, 0.88, 0.86, 0.91, 0.94, 0.95, 0.98],
            ),
            GalleryKpi::new(
                "TIME SAVED",
                "6.8h",
                vec![0.18, 0.28, 0.35, 0.42, 0.55, 0.61, 0.70, 0.76, 0.82, 0.90],
            ),
            GalleryKpi::new(
                "ESCALATED",
                "11",
                vec![0.64, 0.58, 0.62, 0.50, 0.48, 0.44, 0.38, 0.40, 0.34, 0.30],
            ),
        ])
        .with_steps(vec![
            "Classify intent, plan tier and severity from customer thread".into(),
            "Search product docs and past resolved Linear issues".into(),
            "Draft grounded reply or page on-call engineer in Slack".into(),
        ])
        .with_skills(vec![
            "Zendesk".into(),
            "Linear".into(),
            "Slack".into(),
            "Notion".into(),
        ])
        .with_guardrail(
            "Escalate P0 billing or security incidents immediately without auto-closing.",
        ),
        GalleryItem::new(
            "lead-enrichment",
            "Lead enrichment",
            "Researches new signups, scores ICP fit and briefs the account executive.",
            rgb(0x5b86c6).into(),
            rgb(0xc2d6f2).into(),
        )
        .with_trigger("Workspace created in Supabase")
        .with_owner("Theo Park")
        .with_synced_ago("5m ago")
        .with_kpis([
            GalleryKpi::new(
                "RUNS",
                "264",
                vec![0.30, 0.36, 0.44, 0.40, 0.54, 0.62, 0.68, 0.74, 0.80, 0.88],
            ),
            GalleryKpi::new(
                "SUCCESS",
                "96%",
                vec![0.68, 0.74, 0.72, 0.80, 0.78, 0.85, 0.88, 0.90, 0.89, 0.94],
            ),
            GalleryKpi::new(
                "TIME SAVED",
                "4.5h",
                vec![0.25, 0.32, 0.40, 0.48, 0.52, 0.60, 0.66, 0.72, 0.78, 0.84],
            ),
            GalleryKpi::new(
                "ESCALATED",
                "4",
                vec![0.52, 0.48, 0.44, 0.50, 0.38, 0.34, 0.36, 0.28, 0.24, 0.22],
            ),
        ])
        .with_steps(vec![
            "Verify company domain, headcount and engineering stack".into(),
            "Score ICP tier against enterprise qualification rubric".into(),
            "Update HubSpot record and post brief in #new-signups".into(),
        ])
        .with_skills(vec![
            "HubSpot".into(),
            "Slack".into(),
            "Gmail".into(),
            "Supabase".into(),
        ])
        .with_guardrail("Never send outbound cold email before AE reviews the account brief."),
        GalleryItem::new(
            "meeting-recap",
            "Meeting recap",
            "Turns customer calls into action items, CRM updates and follow-up drafts.",
            rgb(0xb07aa4).into(),
            rgb(0xe3c8dc).into(),
        )
        .with_trigger("Call recording finishes in Zoom")
        .with_owner("Nora Patel")
        .with_synced_ago("8m ago")
        .with_kpis([
            GalleryKpi::new(
                "RUNS",
                "95",
                vec![0.24, 0.38, 0.34, 0.50, 0.58, 0.54, 0.68, 0.74, 0.82, 0.86],
            ),
            GalleryKpi::new(
                "SUCCESS",
                "98%",
                vec![0.78, 0.82, 0.84, 0.88, 0.86, 0.90, 0.92, 0.94, 0.96, 0.98],
            ),
            GalleryKpi::new(
                "TIME SAVED",
                "5.1h",
                vec![0.20, 0.30, 0.42, 0.46, 0.58, 0.64, 0.70, 0.78, 0.84, 0.90],
            ),
            GalleryKpi::new(
                "ESCALATED",
                "2",
                vec![0.44, 0.40, 0.38, 0.32, 0.34, 0.28, 0.24, 0.22, 0.20, 0.16],
            ),
        ])
        .with_steps(vec![
            "Extract decisions, blockers and promised delivery dates".into(),
            "Sync deal stage notes to Notion and Linear issues".into(),
            "Stage follow-up email draft in Gmail for host approval".into(),
        ])
        .with_skills(vec![
            "Zoom".into(),
            "Notion".into(),
            "Linear".into(),
            "Gmail".into(),
        ])
        .with_guardrail("Keep internal pricing discussions out of customer-facing recap drafts."),
        GalleryItem::new(
            "churn-watch",
            "Churn watch",
            "Spots accounts that go quiet and suggests a save play.",
            rgb(0xe88d48).into(),
            rgb(0xf6cca8).into(),
        )
        .with_trigger("Usage drops 30%")
        .with_owner("Lena Hart")
        .with_synced_ago("3m ago")
        .with_kpis([
            GalleryKpi::new(
                "RUNS",
                "183",
                vec![0.38, 0.62, 0.16, 0.38, 0.62, 0.84, 0.34, 0.58, 0.80, 0.28],
            ),
            GalleryKpi::new(
                "SUCCESS",
                "97%",
                vec![0.18, 0.86, 0.74, 0.62, 0.50, 0.38, 0.26, 0.16, 0.80, 0.68],
            ),
            GalleryKpi::new(
                "TIME SAVED",
                "3.3h",
                vec![0.24, 0.60, 0.18, 0.54, 0.12, 0.48, 0.86, 0.42, 0.80, 0.36],
            ),
            GalleryKpi::new(
                "ESCALATED",
                "7",
                vec![0.24, 0.32, 0.40, 0.48, 0.56, 0.62, 0.70, 0.78, 0.84, 0.90],
            ),
        ])
        .with_steps(vec![
            "Compare 14-day active seats and API volume against baseline".into(),
            "Pull recent support sentiment and open renewal date from Stripe".into(),
            "Recommend a tailored check-in playbook in #customer-health".into(),
        ])
        .with_skills(vec![
            "Stripe".into(),
            "HubSpot".into(),
            "Slack".into(),
            "Gmail".into(),
        ])
        .with_guardrail("Cannot offer discounts above 15% without VP Sales approval."),
        GalleryItem::new(
            "onboarding-coach",
            "Onboarding coach",
            "Guides new workspaces through setup milestones and nudges stalled steps.",
            rgb(0x58a096).into(),
            rgb(0xbde0da).into(),
        )
        .with_trigger("Trial day 3 without first integration")
        .with_owner("Lena Hart")
        .with_synced_ago("12m ago")
        .with_kpis([
            GalleryKpi::new(
                "RUNS",
                "148",
                vec![0.26, 0.34, 0.42, 0.48, 0.54, 0.62, 0.66, 0.74, 0.80, 0.86],
            ),
            GalleryKpi::new(
                "SUCCESS",
                "94%",
                vec![0.66, 0.70, 0.74, 0.78, 0.76, 0.82, 0.86, 0.88, 0.91, 0.94],
            ),
            GalleryKpi::new(
                "TIME SAVED",
                "2.9h",
                vec![0.22, 0.28, 0.36, 0.44, 0.50, 0.58, 0.64, 0.70, 0.76, 0.82],
            ),
            GalleryKpi::new(
                "ESCALATED",
                "5",
                vec![0.50, 0.46, 0.44, 0.40, 0.38, 0.36, 0.32, 0.30, 0.28, 0.24],
            ),
        ])
        .with_steps(vec![
            "Check connector status and first workflow milestone".into(),
            "Match workspace stack to a 3-minute setup template".into(),
            "Offer a calendar slot with solutions engineering if blocked".into(),
        ])
        .with_skills(vec![
            "Slack".into(),
            "Google Calendar".into(),
            "Gmail".into(),
            "Stripe".into(),
        ])
        .with_guardrail("Send at most one check-in note every 72 hours per trial workspace."),
        GalleryItem::new(
            "content-review",
            "Content review",
            "Checks drafts against brand voice, product claims and legal terminology.",
            rgb(0x7d85a6).into(),
            rgb(0xcfd3e6).into(),
        )
        .with_trigger("Doc moved to Ready for review")
        .with_owner("Devon Cole")
        .with_synced_ago("19m ago")
        .with_kpis([
            GalleryKpi::new(
                "RUNS",
                "76",
                vec![0.20, 0.28, 0.34, 0.40, 0.48, 0.56, 0.62, 0.70, 0.76, 0.82],
            ),
            GalleryKpi::new(
                "SUCCESS",
                "99%",
                vec![0.82, 0.84, 0.88, 0.90, 0.92, 0.94, 0.95, 0.96, 0.98, 0.99],
            ),
            GalleryKpi::new(
                "TIME SAVED",
                "2.4h",
                vec![0.18, 0.24, 0.32, 0.38, 0.46, 0.52, 0.60, 0.66, 0.72, 0.78],
            ),
            GalleryKpi::new(
                "ESCALATED",
                "3",
                vec![0.38, 0.36, 0.34, 0.30, 0.28, 0.26, 0.24, 0.22, 0.20, 0.18],
            ),
        ])
        .with_steps(vec![
            "Verify benchmark numbers against approved release metrics".into(),
            "Highlight unapproved competitor claims or legal phrasing".into(),
            "Post inline revision suggestions in Notion".into(),
        ])
        .with_skills(vec![
            "Notion".into(),
            "Google Drive".into(),
            "Figma".into(),
            "Slack".into(),
        ])
        .with_guardrail("Flag unverified benchmark numbers instead of rewriting customer quotes."),
        GalleryItem::new(
            "release-notes",
            "Release notes",
            "Collects merged pull requests into customer-ready changelog entries.",
            rgb(0x937ab3).into(),
            rgb(0xd7cbe7).into(),
        )
        .with_trigger("Release tag pushed on GitHub")
        .with_owner("Samir Rao")
        .with_synced_ago("27m ago")
        .with_kpis([
            GalleryKpi::new(
                "RUNS",
                "52",
                vec![0.18, 0.26, 0.32, 0.40, 0.46, 0.54, 0.62, 0.68, 0.76, 0.84],
            ),
            GalleryKpi::new(
                "SUCCESS",
                "100%",
                vec![0.90, 0.92, 0.94, 0.95, 0.96, 0.98, 0.98, 0.99, 1.0, 1.0],
            ),
            GalleryKpi::new(
                "TIME SAVED",
                "3.8h",
                vec![0.24, 0.32, 0.40, 0.48, 0.56, 0.62, 0.70, 0.76, 0.84, 0.90],
            ),
            GalleryKpi::new(
                "ESCALATED",
                "1",
                vec![0.30, 0.28, 0.26, 0.24, 0.22, 0.20, 0.18, 0.16, 0.14, 0.12],
            ),
        ])
        .with_steps(vec![
            "Group merged GitHub PRs by user-facing capability".into(),
            "Link resolved Linear issues and strip internal refactors".into(),
            "Open changelog draft in Notion for product sign-off".into(),
        ])
        .with_skills(vec![
            "GitHub".into(),
            "Linear".into(),
            "Notion".into(),
            "Slack".into(),
        ])
        .with_guardrail("Omit internal refactor PRs and unreleased feature-flagged endpoints."),
    ]
}

fn translate_agent_name<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    if UiLanguage::current(cx) == UiLanguage::English {
        return fallback;
    }
    match id {
        "support-triage" => "工单分诊",
        "lead-enrichment" => "线索补全",
        "meeting-recap" => "会议纪要",
        "churn-watch" => "流失预警",
        "onboarding-coach" => "入驻向导",
        "content-review" => "内容审校",
        "release-notes" => "发版说明",
        _ => fallback,
    }
}

fn translate_agent_desc<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    if UiLanguage::current(cx) == UiLanguage::English {
        return fallback;
    }
    match id {
        "support-triage" => "自动分类客户工单、起草有据可查的回复并转交紧急缺陷。",
        "lead-enrichment" => "调研新注册团队、评估理想客户匹配度并向销售同步简报。",
        "meeting-recap" => "将客户通话转化为行动项、CRM 更新记录与跟进邮件草稿。",
        "churn-watch" => "识别活跃度下滑的客户账户并推荐挽留剧本。",
        "onboarding-coach" => "引导新工作区完成配置里程碑并温和提醒停滞步骤。",
        "content-review" => "对照品牌语气、产品数据口径与法务术语审校文稿。",
        "release-notes" => "汇总已合并的拉取请求并整理为面向客户的更新日志。",
        _ => fallback,
    }
}

fn translate_trigger<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    if UiLanguage::current(cx) == UiLanguage::English {
        return fallback;
    }
    match id {
        "support-triage" => "Zendesk 收到新工单",
        "lead-enrichment" => "Supabase 创建新工作区",
        "meeting-recap" => "Zoom 通话录制完成",
        "churn-watch" => "用量下降 30%",
        "onboarding-coach" => "试用第 3 天仍未接入集成",
        "content-review" => "文档移至待审校状态",
        "release-notes" => "GitHub 推送发版标签",
        _ => fallback,
    }
}

fn translate_kpi_label<'a>(label: &'a str, cx: &App) -> &'a str {
    if UiLanguage::current(cx) == UiLanguage::English {
        return label;
    }
    match label {
        "RUNS" => "运行次数",
        "SUCCESS" => "成功率",
        "TIME SAVED" => "节省工时",
        "ESCALATED" => "人工升级",
        _ => label,
    }
}

/// Feature-section gallery block showcasing expandable gradient agent panels and a live product screen.
pub struct AgentGalleryBlock {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    subtitle: Option<SharedString>,
    items: Vec<GalleryItem>,
    custom_items: bool,
    active_index: usize,
    screen_tab: GalleryScreenTab,
    pos_slide: Slide,
    open_slides: Vec<Slide>,
    hover_slides: Vec<Slide>,
    user_notes: Vec<(SharedString, SharedString)>,
    composer_editor: Option<Entity<Editor>>,
    search_filter_all: bool,
    loading: bool,
    empty: bool,
    toast: Option<(SharedString, Instant)>,
    clock: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<AgentGalleryEvent> for AgentGalleryBlock {}

impl AgentGalleryBlock {
    /// Construct an [`AgentGalleryBlock`] with 7 playbook agents and the middle panel (`Churn watch`) open.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let items = default_gallery_items();
        let active_index = items.len() / 2;
        let open_slides = (0..items.len())
            .map(|idx| Slide::new(if idx == active_index { 1. } else { 0. }))
            .collect();
        let hover_slides = (0..items.len()).map(|_| Slide::snappy(0.)).collect();

        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            title: None,
            subtitle: None,
            items,
            custom_items: false,
            active_index,
            screen_tab: GalleryScreenTab::Context,
            pos_slide: Slide::new(active_index as f32),
            open_slides,
            hover_slides,
            user_notes: Vec::new(),
            composer_editor: None,
            search_filter_all: true,
            loading: false,
            empty: false,
            toast: None,
            clock: Instant::now(),
            measured_width: Rc::new(Cell::new(894.)),
        }
    }

    /// Override the section heading displayed above the panels.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override the section subtitle displayed beneath the heading.
    pub fn with_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Replace the gallery panels and reset open state to the middle item unless already in bounds.
    pub fn with_items(mut self, items: Vec<GalleryItem>) -> Self {
        self.custom_items = true;
        self.active_index = if items.is_empty() { 0 } else { items.len() / 2 };
        self.pos_slide = Slide::new(self.active_index as f32);
        self.open_slides = (0..items.len())
            .map(|idx| Slide::new(if idx == self.active_index { 1. } else { 0. }))
            .collect();
        self.hover_slides = (0..items.len()).map(|_| Slide::snappy(0.)).collect();
        self.items = items;
        self
    }

    /// Choose which panel index starts open (`items.len() / 2` by default).
    pub fn with_default_index(mut self, index: usize) -> Self {
        if !self.items.is_empty() {
            self.active_index = index.min(self.items.len() - 1);
            self.pos_slide.reset(self.active_index as f32);
            for (idx, slide) in self.open_slides.iter_mut().enumerate() {
                slide.reset(if idx == self.active_index { 1. } else { 0. });
            }
        }
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

    /// Return the index of the currently open gallery panel.
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// Return the gallery items currently configured on this block.
    pub fn items(&self) -> &[GalleryItem] {
        &self.items
    }

    /// Smoothly glide the panel at `index` open into the center showcase.
    pub fn select_index(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let target = index.min(self.items.len() - 1);
        if self.active_index == target {
            return;
        }
        self.active_index = target;
        self.pos_slide.duration(target as f32, 0.46, cx);
        for (idx, slide) in self.open_slides.iter_mut().enumerate() {
            slide.duration(if idx == target { 1. } else { 0. }, 0.46, cx);
        }
        let id = self.items[target].id.clone();
        cx.emit(AgentGalleryEvent::Selected { index: target, id });
        cx.notify();
    }

    /// Advance to the next agent panel (wrapping around from last to first).
    pub fn select_next(&mut self, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let next = (self.active_index + 1) % self.items.len();
        self.select_index(next, cx);
    }

    /// Move to the previous agent panel (wrapping around from first to last).
    pub fn select_prev(&mut self, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let prev = if self.active_index == 0 {
            self.items.len() - 1
        } else {
            self.active_index - 1
        };
        self.select_index(prev, cx);
    }

    /// Toggle skeleton loading placeholders at runtime.
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

    /// Toggle the zero-items empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the empty state is forced on.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    fn ensure_composer(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let ph: SharedString = tr(
            cx,
            "Add context, e.g. \"Cannot offer discounts\"",
            "添加上下文，例如“不可直接提供折扣”",
        )
        .into();
        if let Some(ed) = &self.composer_editor {
            ed.update(cx, |e, _| e.set_placeholder(ph));
            return;
        }
        let ed = cx.new(|cx| {
            let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
            e.set_placeholder(ph);
            e.font_size = 12.5;
            e
        });
        cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
            EditEvent::Submitted => {
                this.submit_context(w, cx);
            }
            EditEvent::Escape => {
                this.focus.focus(w, cx);
                cx.notify();
            }
            EditEvent::Changed => {
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.composer_editor = Some(ed);
    }

    fn submit_context(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.composer_editor.clone() else {
            return;
        };
        let raw = ed.read(cx).value().trim().to_owned();
        if raw.is_empty() || self.items.is_empty() {
            return;
        }
        let item_id = self.items[self.active_index].id.clone();
        let text: SharedString = raw.into();
        self.user_notes.insert(0, (item_id.clone(), text.clone()));
        ed.update(cx, |e, cx| e.set_value("", w, cx));
        self.toast = Some((
            tr(cx, "Added to agent context", "已添加到智能体上下文").into(),
            Instant::now(),
        ));
        cx.emit(AgentGalleryEvent::ContextAdded { item_id, text });
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(ed) = &self.composer_editor
            && ed.read(cx).focus.is_focused(_w)
        {
            return;
        }
        match ev.keystroke.key.as_str() {
            "left" => {
                self.select_prev(cx);
                cx.stop_propagation();
            }
            "right" => {
                self.select_next(cx);
                cx.stop_propagation();
            }
            "home" => {
                self.select_index(0, cx);
                cx.stop_propagation();
            }
            "end" if !self.items.is_empty() => {
                self.select_index(self.items.len() - 1, cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }

    fn render_panel_artwork(
        strong: Hsla,
        soft: Hsla,
        saturation: f32,
        dark: bool,
    ) -> impl IntoElement {
        let sat = saturation.clamp(0.10, 1.0);
        let base_top: Hsla = if dark {
            rgb(0x202023).into()
        } else {
            rgb(0xf6f6f7).into()
        };
        let base_bottom: Hsla = if dark {
            rgb(0x18181a).into()
        } else {
            rgb(0xededf0).into()
        };
        let neutral_mid = pro_mix_color(base_top, base_bottom, 0.5);
        let strong_tinted = pro_mix_color(neutral_mid, strong, sat);
        let soft_tinted = pro_mix_color(neutral_mid, soft, sat);

        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                if bounds.size.width <= px(1.) || bounds.size.height <= px(1.) {
                    return;
                }
                let r = px(12.);
                // 1. Base vertical surface gradient
                window.paint_quad(
                    fill(
                        bounds,
                        linear_gradient(
                            180.,
                            linear_color_stop(base_top, 0.),
                            linear_color_stop(base_bottom, 1.),
                        ),
                    )
                    .corner_radii(r),
                );
                // 2. Top-left signature wash
                window.paint_quad(
                    fill(
                        bounds,
                        linear_gradient(
                            132.,
                            linear_color_stop(
                                strong_tinted.opacity(if dark { 0.42 * sat } else { 0.48 * sat }),
                                0.,
                            ),
                            linear_color_stop(transparent_black(), 0.62),
                        ),
                    )
                    .corner_radii(r),
                );
                // 3. Bottom-right soft atmospheric wash
                window.paint_quad(
                    fill(
                        bounds,
                        linear_gradient(
                            318.,
                            linear_color_stop(
                                soft_tinted.opacity(if dark { 0.22 * sat } else { 0.68 * sat }),
                                0.,
                            ),
                            linear_color_stop(transparent_black(), 0.68),
                        ),
                    )
                    .corner_radii(r),
                );
                // 4. Diagonal specular ribbon sheen
                let sheen_col = if dark {
                    hsla(0., 0., 1., 0.055)
                } else {
                    hsla(0., 0., 1., 0.62)
                };
                window.paint_quad(
                    fill(
                        bounds,
                        linear_gradient(
                            118.,
                            linear_color_stop(transparent_black(), 0.36),
                            linear_color_stop(sheen_col, 0.50),
                        ),
                    )
                    .corner_radii(r),
                );
            },
        )
        .absolute()
        .inset_0()
    }

    fn render_mini_sparkline(points: &[f32], stroke: Hsla, dot: Hsla) -> impl IntoElement {
        let pts = points.to_vec();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4. || bh <= 4. || pts.len() < 2 {
                    return;
                }
                let pad_x = 2.5;
                let pad_y = 2.5;
                let map_pt = |idx: usize, val: f32| -> Point<Pixels> {
                    let tx = idx as f32 / (pts.len() - 1) as f32;
                    let ty = 1. - val.clamp(0., 1.);
                    point(
                        bounds.left() + px(pad_x + tx * (bw - 2. * pad_x)),
                        bounds.top() + px(pad_y + ty * (bh - 2. * pad_y)),
                    )
                };
                let mut builder = PathBuilder::stroke(px(1.35));
                let mut last_pt = map_pt(0, pts[0]);
                for (i, &v) in pts.iter().enumerate() {
                    let pt = map_pt(i, v);
                    if i == 0 {
                        builder.move_to(pt);
                    } else {
                        builder.line_to(pt);
                    }
                    last_pt = pt;
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, stroke);
                }
                let dot_bounds = Bounds::new(
                    point(last_pt.x - px(1.8), last_pt.y - px(1.8)),
                    size(px(3.6), px(3.6)),
                );
                window.paint_quad(fill(dot_bounds, dot).corner_radii(px(99.)));
            },
        )
        .w(px(38.))
        .h(px(15.))
        .flex_none()
    }

    fn render_slim_label(
        label: &str,
        hovered: bool,
        far: bool,
        compact: bool,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        let fg = if hovered {
            p.ink
        } else if far {
            p.muted
        } else {
            p.text
        };
        if UiLanguage::current(cx) == UiLanguage::Chinese {
            let mut col = div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(2.))
                .text_size(px(if compact { 11.5 } else { 12.5 }))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg);
            for ch in label.chars() {
                col = col.child(div().line_height(px(15.)).child(ch.to_string()));
            }
            col.into_any_element()
        } else {
            let mut col = div()
                .px(px(if compact { 4. } else { 6. }))
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(1.))
                .text_size(px(if compact { 10.5 } else { 11.5 }))
                .line_height(px(if compact { 13.5 } else { 15. }))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg);
            for word in label.split_whitespace() {
                col = col.child(
                    div()
                        .w_full()
                        .truncate()
                        .text_center()
                        .child(word.to_owned()),
                );
            }
            col.into_any_element()
        }
    }

    fn render_product_screen(
        &self,
        item: &GalleryItem,
        open_amt: f32,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let screen_card: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let screen_t = if disabled_motion(cx) {
            1.0
        } else {
            ((open_amt - 0.30) / 0.70).clamp(0., 1.)
        };
        if screen_t <= 0.01 {
            return div().into_any_element();
        }

        let display_name: SharedString = if self.custom_items {
            item.name.clone()
        } else {
            translate_agent_name(&item.id, &item.name, cx).into()
        };
        let display_trigger: SharedString = if self.custom_items {
            item.trigger.clone()
        } else {
            translate_trigger(&item.id, &item.trigger, cx).into()
        };
        let sub_line: SharedString = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("触发条件：“{display_trigger}” · 负责人 {}", item.owner).into()
        } else {
            format!("Triggered by “{display_trigger}” · Owner {}", item.owner).into()
        };

        let [strong_c, soft_c] = item.colors;
        let small_mark = div()
            .size(px(14.))
            .rounded(px(4.))
            .flex_none()
            .bg(linear_gradient(
                135.,
                linear_color_stop(strong_c, 0.),
                linear_color_stop(soft_c, 1.),
            ))
            .border_1()
            .border_color(hsla(0., 0., 0., 0.08));

        let hero_logo = div()
            .size(px(28.))
            .rounded(px(7.))
            .flex_none()
            .bg(linear_gradient(
                135.,
                linear_color_stop(strong_c, 0.),
                linear_color_stop(soft_c, 1.),
            ))
            .border_1()
            .border_color(hsla(0., 0., 0., 0.08));

        let brand_chip = div()
            .size(px(20.))
            .rounded(px(6.))
            .flex_none()
            .bg(p.primary)
            .flex()
            .items_center()
            .justify_center()
            .children(pro_app_artwork("kense", 12., cx));

        let switcher = activate(
            div()
                .id(pro_eid(&self.id, "screen-switcher"))
                .role(Role::Button)
                .aria_label(tr(cx, "Switch showcase agent", "切换展示智能体"))
                .tab_index(0)
                .h(px(26.))
                .pl(px(6.))
                .pr(px(7.))
                .rounded(px(6.))
                .bg(screen_card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_color(p.focus))
                .child(small_mark)
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .truncate()
                        .child(display_name.clone()),
                )
                .child(pro_svg("0 0 24 24", CHEVRON_UP_DOWN_SVG, 11., p.muted)),
            cx,
            |this, _, cx| {
                this.select_next(cx);
            },
        );

        let new_agent_btn = activate(
            div()
                .id(pro_eid(&self.id, "screen-new-agent"))
                .role(Role::Button)
                .aria_label(tr(cx, "New agent", "新建智能体"))
                .tab_index(0)
                .h(px(26.))
                .px(px(9.))
                .rounded(px(6.))
                .bg(p.primary)
                .text_color(p.on_primary)
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .gap(px(5.))
                .cursor_pointer()
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(pro_icon(ProIcon::Plus, 12., p.on_primary))
                .child(tr(cx, "New agent", "新建智能体")),
            cx,
            |this, _, cx| {
                this.toast = Some((
                    tr(cx, "Opened new agent template", "已打开新建智能体模板").into(),
                    Instant::now(),
                ));
                cx.emit(AgentGalleryEvent::NewAgentRequested);
                cx.notify();
            },
        );

        let bar = div()
            .h(px(44.))
            .px(px(14.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(brand_chip)
                    .child(switcher),
            )
            .child(new_agent_btn);

        let hero = div()
            .flex()
            .items_center()
            .justify_between()
            .child(hero_logo)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(pro_icon(ProIcon::Refresh, 11., p.faint))
                    .child(item.synced_ago.clone()),
            );

        let kpi_count = if compact { 2 } else { 4 };
        let mut kpi_row = div().mt(px(16.)).flex().items_center().gap(px(14.));
        for kpi in item.kpis.iter().take(kpi_count) {
            kpi_row = kpi_row.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(translate_kpi_label(&kpi.label, cx).to_owned()),
                            )
                            .child(
                                div()
                                    .mt(px(1.))
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(kpi.value.clone()),
                            ),
                    )
                    .child(Self::render_mini_sparkline(&kpi.spark, p.faint, p.muted)),
            );
        }

        let tab_sensor = pro_selection_sensor();
        let thumb_fill: Hsla = if p.dark {
            rgb(0x2c2c2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut tabs_bar = div()
            .mt(px(16.))
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .relative()
            .flex()
            .items_center()
            .self_start()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "screen-tabs-pill"),
                self.screen_tab.key(),
                tab_sensor.clone(),
                6.,
                thumb_fill,
                Some(p.line),
                true,
            ));

        for tab in GalleryScreenTab::ALL {
            let active = self.screen_tab == tab;
            let item_id = item.id.clone();
            tabs_bar = tabs_bar.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("screen-tab-{}", tab.key())))
                    .role(Role::Tab)
                    .aria_selected(active)
                    .tab_index(0)
                    .relative()
                    .h(px(25.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.ink } else { p.text })
                    .cursor_pointer()
                    .when(active, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .child(tab.label(cx)),
                cx,
                move |this, _, cx| {
                    if this.screen_tab != tab {
                        this.screen_tab = tab;
                        cx.emit(AgentGalleryEvent::TabChanged {
                            item_id: item_id.clone(),
                            tab,
                        });
                        cx.notify();
                    }
                },
            ));
        }

        let tab_body = match self.screen_tab {
            GalleryScreenTab::Context => self.render_context_tab(item, screen_card, &p, cx),
            GalleryScreenTab::Runs => self.render_runs_tab(item, screen_card, &p, cx),
            GalleryScreenTab::Tools => self.render_tools_tab(item, screen_card, &p, cx),
            GalleryScreenTab::Settings => self.render_settings_tab(item, screen_card, &p, cx),
        };

        let page = div()
            .px(px(if compact { 18. } else { 26. }))
            .pt(px(18.))
            .pb(px(22.))
            .flex()
            .flex_col()
            .child(hero)
            .child(
                div()
                    .mt(px(10.))
                    .text_size(px(if compact { 18. } else { 21. }))
                    .line_height(px(if compact { 24. } else { 27. }))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(display_name),
            )
            .child(
                div()
                    .mt(px(2.))
                    .text_size(px(11.5))
                    .line_height(px(16.))
                    .text_color(p.muted)
                    .truncate()
                    .child(sub_line),
            )
            .child(kpi_row)
            .child(tabs_bar)
            .child(pro_view_transition(
                pro_eid(&self.id, "screen-tab-vtx"),
                format!("{}-{}", item.id, self.screen_tab.key()),
                tab_body,
            ));

        let inset_left = if compact { 18. } else { 34. };
        let inset_top = if compact { 24. } else { 32. };

        div()
            .absolute()
            .top(px(inset_top + 12. * (1. - screen_t)))
            .left(px(inset_left + 18. * (1. - screen_t)))
            .right(px(-16.))
            .bottom(px(-16.))
            .opacity(screen_t)
            .rounded(px(12.))
            .bg(screen_card)
            .border_t_1()
            .border_l_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(bar)
            .child(page)
            .into_any_element()
    }

    fn render_context_tab(
        &self,
        item: &GalleryItem,
        screen_card: Hsla,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ed_click = self.composer_editor.clone();
        let filter_label = if self.search_filter_all {
            tr(cx, "All", "全部")
        } else {
            tr(cx, "Rules", "规则")
        };

        let toolbar = div()
            .mt(px(12.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(27.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(screen_card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(pro_icon(ProIcon::Search, 12., p.muted))
                    .child(
                        div()
                            .truncate()
                            .child(tr(cx, "Search context...", "搜索上下文...")),
                    ),
            )
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "screen-ctx-filter"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Toggle context filter", "切换上下文筛选"))
                    .tab_index(0)
                    .h(px(27.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(screen_card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(pro_icon(ProIcon::Filter, 12., p.muted))
                    .child(filter_label),
                cx,
                |this, _, cx| {
                    this.search_filter_all = !this.search_filter_all;
                    cx.notify();
                },
            ));

        let composer_box = div()
            .flex_1()
            .min_w_0()
            .rounded(px(9.))
            .bg(screen_card)
            .border_1()
            .border_color(p.line)
            .px(px(10.))
            .pt(px(8.))
            .pb(px(7.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .cursor_text()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, w, cx| {
                    if let Some(ed) = &ed_click {
                        ed.read(cx).focus.clone().focus(w, cx);
                    }
                }),
            )
            .when_some(self.composer_editor.clone(), |d, ed| {
                d.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .h(px(36.))
                        .overflow_hidden()
                        .child(ed),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(pro_icon(ProIcon::Paperclip, 13., p.muted))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "screen-add-ctx"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Add context", "添加上下文"))
                            .tab_index(0)
                            .h(px(24.))
                            .px(px(8.))
                            .rounded(px(5.))
                            .bg(screen_card)
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(tr(cx, "Add context", "添加上下文")),
                        cx,
                        |this, w, cx| {
                            this.submit_context(w, cx);
                        },
                    )),
            );

        let mut feed = div().mt(px(12.)).flex().flex_col().gap(px(10.)).child(
            div()
                .flex()
                .items_start()
                .gap(px(10.))
                .child(pro_person_avatar(&item.owner, 24., false, None, cx))
                .child(composer_box),
        );

        for (idx, (note_item_id, note_text)) in self.user_notes.iter().enumerate() {
            if note_item_id == &item.id {
                feed = feed.child(
                    div()
                        .id(pro_eid(&self.id, format!("unote-{idx}")))
                        .flex()
                        .items_start()
                        .gap(px(10.))
                        .child(
                            div()
                                .size(px(24.))
                                .rounded_full()
                                .bg(p.chip)
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::Note, 12., p.muted)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .rounded(px(8.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line)
                                .px(px(10.))
                                .py(px(6.))
                                .text_size(px(12.))
                                .text_color(p.ink)
                                .child(note_text.clone()),
                        ),
                );
            }
        }

        feed = feed.child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .size(px(24.))
                        .rounded_full()
                        .bg(p.chip)
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Shield, 12., p.muted)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .truncate()
                        .child(item.guardrail.clone()),
                ),
        );

        div()
            .mt(px(16.))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Agent context", "智能体上下文")),
            )
            .child(
                div()
                    .mt(px(1.))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(tr(
                        cx,
                        "Goals, files and rules the agent uses on every run.",
                        "智能体每次运行时遵循的目标、文件与护栏规则。",
                    )),
            )
            .child(toolbar)
            .child(feed)
            .into_any_element()
    }

    fn render_runs_tab(
        &self,
        item: &GalleryItem,
        screen_card: Hsla,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        let mut list = div().mt(px(10.)).flex().flex_col().gap(px(6.));
        let steps_iter: Vec<SharedString> = if item.steps.is_empty() {
            vec![
                tr(
                    cx,
                    "Evaluate incoming trigger payload",
                    "评估传入的触发载荷",
                )
                .into(),
                tr(cx, "Execute connected tool actions", "执行已授权的工具操作").into(),
                tr(
                    cx,
                    "Post audit summary to workspace",
                    "向工作区发布审计摘要",
                )
                .into(),
            ]
        } else {
            item.steps.clone()
        };
        for (idx, step) in steps_iter.into_iter().enumerate() {
            list = list.child(
                div()
                    .rounded(px(8.))
                    .bg(screen_card)
                    .border_1()
                    .border_color(p.line)
                    .px(px(10.))
                    .py(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .size(px(18.))
                            .rounded_full()
                            .bg(p.green_soft)
                            .text_color(p.green_ink)
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(format!("{}", idx + 1)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .truncate()
                            .child(step),
                    ),
            );
        }
        div()
            .mt(px(16.))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Playbook steps", "剧本执行步骤")),
            )
            .child(list)
            .into_any_element()
    }

    fn render_tools_tab(
        &self,
        item: &GalleryItem,
        screen_card: Hsla,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        let tools: Vec<SharedString> = if item.skills.is_empty() {
            vec![
                "Slack".into(),
                "Linear".into(),
                "Notion".into(),
                "Gmail".into(),
            ]
        } else {
            item.skills.clone()
        };
        let mut grid = div().mt(px(10.)).flex().flex_wrap().gap(px(8.));
        for tool in tools {
            grid = grid.child(
                div()
                    .h(px(34.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .bg(screen_card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_app_tile(&tool, 20., cx))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tool),
                    )
                    .child(div().size(px(6.)).rounded_full().bg(p.green)),
            );
        }
        div()
            .mt(px(16.))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Connected skills & tools", "已连接技能与工具")),
            )
            .child(grid)
            .into_any_element()
    }

    fn render_settings_tab(
        &self,
        item: &GalleryItem,
        screen_card: Hsla,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        div()
            .mt(px(16.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Safety guardrail", "安全护栏限制")),
            )
            .child(
                div()
                    .rounded(px(8.))
                    .bg(screen_card)
                    .border_1()
                    .border_color(p.line)
                    .p(px(10.))
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(pro_icon(ProIcon::Shield, 14., p.amber))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .text_color(p.ink)
                            .child(item.guardrail.clone()),
                    ),
            )
            .into_any_element()
    }

    fn render_loading(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        if !disabled_motion(cx) {
            w.request_animation_frame();
        }
        let phase = (self.clock.elapsed().as_secs_f32() * 0.75).fract();
        div()
            .w_full()
            .py(px(16.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(24.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(260., 22., 6., phase, cx))
                    .child(pro_skeleton_bar(360., 14., 5., phase + 0.12, cx)),
            )
            .child(
                div()
                    .w_full()
                    .h(px(380.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .w(px(72.))
                            .h_full()
                            .rounded(px(12.))
                            .bg(p.chip)
                            .border_1()
                            .border_color(p.line),
                    )
                    .child(
                        div()
                            .w(px(440.))
                            .h_full()
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .p(px(24.))
                            .flex()
                            .flex_col()
                            .gap(px(14.))
                            .child(pro_skeleton_bar(160., 20., 6., phase + 0.2, cx))
                            .child(pro_skeleton_bar(280., 14., 5., phase + 0.3, cx))
                            .child(pro_skeleton_bar(360., 96., 8., phase + 0.4, cx)),
                    )
                    .child(
                        div()
                            .w(px(72.))
                            .h_full()
                            .rounded(px(12.))
                            .bg(p.chip)
                            .border_1()
                            .border_color(p.line),
                    ),
            )
    }
}

impl Render for AgentGalleryBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_composer(w, cx);
        let p = ProPalette::get(cx);
        let measure_cell = self.measured_width.clone();
        let container_w = measure_cell.get().max(320.);
        let narrow = container_w <= 760.;
        let ultra_narrow = container_w <= 460.;

        let sensor = canvas(
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
        .h(px(0.));

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .relative()
                .child(sensor)
                .child(self.render_loading(w, cx))
                .into_any_element();
        }

        if self.empty || self.items.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .relative()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(sensor)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No agents in this gallery", "暂无可展示的智能体"),
                    tr(
                        cx,
                        "Add playbook items to preview each agent's workflow screen.",
                        "添加智能体剧本项即可在此预览完整工作台界面。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-defaults"),
                        tr(cx, "Load sample agents", "载入示例智能体").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.items.is_empty() {
                                this.items = default_gallery_items();
                                this.custom_items = false;
                                this.active_index = this.items.len() / 2;
                                this.pos_slide = Slide::new(this.active_index as f32);
                                this.open_slides = (0..this.items.len())
                                    .map(|i| {
                                        Slide::new(if i == this.active_index { 1. } else { 0. })
                                    })
                                    .collect();
                                this.hover_slides =
                                    (0..this.items.len()).map(|_| Slide::snappy(0.)).collect();
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let heading_text: SharedString = self.title.clone().unwrap_or_else(|| {
            tr(
                cx,
                "Agents that run your playbook",
                "执行团队剧本的智能体矩阵",
            )
            .into()
        });
        let subtitle_text: SharedString = self.subtitle.clone().unwrap_or_else(|| {
            tr(
                cx,
                "Give each agent its steps, tools and limits – then let it work inside the tools your team already uses.",
                "为每个智能体设定步骤、工具与权限边界，让它们在团队日常使用的软件中协同运转。",
            )
            .into()
        });

        let panel_h = if ultra_narrow {
            356.
        } else if narrow {
            436.
        } else {
            468.
        };
        let side_w = if ultra_narrow {
            38.
        } else if narrow {
            54.
        } else {
            77.
        };
        let gap = if narrow { 8. } else { 10. };
        let open_w = if narrow {
            (container_w - 2. * (side_w + gap) - 20.).max(256.)
        } else {
            (container_w * 0.522).clamp(466., 620.)
        };

        let n = self.items.len();
        let pos = self.pos_slide.get(w, cx);
        let open_amounts: Vec<f32> = self
            .open_slides
            .iter()
            .map(|s| s.get(w, cx).clamp(0., 1.))
            .collect();
        let hover_amounts: Vec<f32> = self
            .hover_slides
            .iter()
            .map(|s| s.get(w, cx).clamp(0., 1.))
            .collect();

        // Flanking wrap-around slim panels (2 left, 2 right) keep the open center panel framed symmetrically
        let flank_count = if n >= 3 { 2usize } else { 0usize };
        let track_left =
            (container_w * 0.5) - ((flank_count as f32 + pos) * (side_w + gap)) - (open_w * 0.5);

        let mut track = div()
            .relative()
            .left(px(track_left))
            .h(px(panel_h))
            .flex()
            .items_stretch()
            .gap(px(gap));

        // Left wrap-around flank panels
        for f in 0..flank_count {
            let real_idx = (n + f - flank_count) % n;
            let item = &self.items[real_idx];
            let label = if self.custom_items {
                item.name.as_ref()
            } else {
                translate_agent_name(&item.id, &item.name, cx)
            };
            track = track.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("flank-l-{f}")))
                    .role(Role::Button)
                    .aria_label(format!("{} {label}", tr(cx, "Show", "展开")))
                    .w(px(side_w))
                    .h_full()
                    .flex_none()
                    .relative()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .cursor_pointer()
                    .child(Self::render_panel_artwork(
                        item.colors[0],
                        item.colors[1],
                        0.16,
                        p.dark,
                    ))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .pb(px(18.))
                            .flex()
                            .items_end()
                            .justify_center()
                            .child(Self::render_slim_label(label, false, true, narrow, &p, cx)),
                    ),
                cx,
                move |this, _, cx| {
                    this.select_index(real_idx, cx);
                },
            ));
        }

        // Main interactive panels (0..n)
        for idx in 0..n {
            let open_amt = open_amounts[idx];
            let hov_amt = hover_amounts[idx];
            let cur_w = side_w + (open_w - side_w) * open_amt;
            let dist = idx.abs_diff(self.active_index);
            let base_sat = if dist == 0 {
                1.0
            } else if dist == 1 {
                0.34
            } else {
                0.15
            };
            let sat = (base_sat + (1.0 - base_sat) * open_amt.max(hov_amt * 0.85)).clamp(0.12, 1.0);

            let item_snapshot = self.items[idx].clone();
            let label_str: SharedString = if self.custom_items {
                item_snapshot.name.clone()
            } else {
                translate_agent_name(&item_snapshot.id, &item_snapshot.name, cx).into()
            };

            let screen_el = if open_amt > 0.02 {
                Some(self.render_product_screen(&item_snapshot, open_amt, ultra_narrow, cx))
            } else {
                None
            };

            let label_alpha = (1.0 - open_amt * 2.5).clamp(0., 1.);
            let is_open = idx == self.active_index;

            let mut panel = div()
                .id(pro_eid(&self.id, format!("panel-{idx}")))
                .w(px(cur_w))
                .h_full()
                .flex_none()
                .relative()
                .rounded(px(12.))
                .border_1()
                .border_color(if is_open { p.line_strong } else { p.line })
                .overflow_hidden()
                .on_hover(cx.listener(move |this, inside: &bool, w, cx| {
                    if let Some(sl) = this.hover_slides.get_mut(idx) {
                        sl.duration(if *inside { 1. } else { 0. }, 0.24, cx);
                        cx.notify();
                        w.refresh();
                    }
                }))
                .child(Self::render_panel_artwork(
                    item_snapshot.colors[0],
                    item_snapshot.colors[1],
                    sat,
                    p.dark,
                ))
                .when_some(screen_el, |d, sc| d.child(sc));

            if !is_open || label_alpha > 0.01 {
                panel = panel.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("pick-{idx}")))
                        .role(Role::Button)
                        .aria_label(format!("{} {label_str}", tr(cx, "Show", "展开")))
                        .tab_index(if is_open { -1 } else { 0 })
                        .absolute()
                        .inset_0()
                        .pb(px(if narrow { 14. } else { 18. }))
                        .opacity(label_alpha)
                        .flex()
                        .items_end()
                        .justify_center()
                        .cursor_pointer()
                        .focus_visible(move |s| s.border_2().border_color(p.focus))
                        .child(Self::render_slim_label(
                            &label_str,
                            hov_amt > 0.35,
                            dist > 1,
                            narrow,
                            &p,
                            cx,
                        )),
                    cx,
                    move |this, _, cx| {
                        this.select_index(idx, cx);
                    },
                ));
            }

            track = track.child(panel);
        }

        // Right wrap-around flank panels
        for f in 0..flank_count {
            let real_idx = f % n;
            let item = &self.items[real_idx];
            let label = if self.custom_items {
                item.name.as_ref()
            } else {
                translate_agent_name(&item.id, &item.name, cx)
            };
            track = track.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("flank-r-{f}")))
                    .role(Role::Button)
                    .aria_label(format!("{} {label}", tr(cx, "Show", "展开")))
                    .w(px(side_w))
                    .h_full()
                    .flex_none()
                    .relative()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .cursor_pointer()
                    .child(Self::render_panel_artwork(
                        item.colors[0],
                        item.colors[1],
                        0.16,
                        p.dark,
                    ))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .pb(px(18.))
                            .flex()
                            .items_end()
                            .justify_center()
                            .child(Self::render_slim_label(label, false, true, narrow, &p, cx)),
                    ),
                cx,
                move |this, _, cx| {
                    this.select_index(real_idx, cx);
                },
            ));
        }

        let active_item = &self.items[self.active_index];
        let active_name: SharedString = if self.custom_items {
            active_item.name.clone()
        } else {
            translate_agent_name(&active_item.id, &active_item.name, cx).into()
        };
        let active_desc: SharedString = if self.custom_items {
            active_item.description.clone()
        } else {
            translate_agent_desc(&active_item.id, &active_item.description, cx).into()
        };

        let caption_el = pro_view_transition(
            pro_eid(&self.id, "caption-vtx"),
            active_item.id.clone(),
            div()
                .flex()
                .flex_wrap()
                .items_baseline()
                .gap_x(px(5.))
                .text_size(px(13.))
                .line_height(px(20.))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(format!("{active_name}.")),
                )
                .child(div().text_color(p.text).child(active_desc.clone())),
        );

        let foot_w = if narrow {
            (container_w - 32.).max(260.)
        } else {
            open_w
        };

        let prev_btn = activate(
            div()
                .id(pro_eid(&self.id, "prev-agent"))
                .role(Role::Button)
                .aria_label(tr(cx, "Previous agent", "上一个智能体"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.chip_hover))
                .focus_visible(move |s| s.border_color(p.focus))
                .child(pro_icon(ProIcon::ChevronLeft, 14., p.ink)),
            cx,
            |this, _, cx| {
                this.select_prev(cx);
            },
        );

        let next_btn = activate(
            div()
                .id(pro_eid(&self.id, "next-agent"))
                .role(Role::Button)
                .aria_label(tr(cx, "Next agent", "下一个智能体"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.chip_hover))
                .focus_visible(move |s| s.border_color(p.focus))
                .child(pro_icon(ProIcon::ChevronRight, 14., p.ink)),
            cx,
            |this, _, cx| {
                this.select_next(cx);
            },
        );

        let footer = div()
            .w(px(foot_w))
            .mx_auto()
            .mt(px(18.))
            .flex()
            .items_start()
            .justify_between()
            .gap(px(16.))
            .child(div().flex_1().min_w_0().max_w(px(460.)).child(caption_el))
            .when(n > 1, |d| {
                d.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(prev_btn)
                        .child(next_btn),
                )
            });

        let sr_announce = format!("{active_name}. {active_desc}");

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(heading_text.clone())
            .w_full()
            .pt(px(if narrow { 20. } else { 32. }))
            .pb(px(if narrow { 20. } else { 28. }))
            .relative()
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.handle_key_down(ev, w, cx);
            }))
            .child(sensor)
            .child(
                div()
                    .max_w(px(560.))
                    .mx_auto()
                    .mb(px(if narrow { 20. } else { 28. }))
                    .px(px(20.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .text_center()
                    .child(
                        div()
                            .text_size(px(if ultra_narrow { 20. } else { 24. }))
                            .line_height(px(if ultra_narrow { 26. } else { 32. }))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(heading_text),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .max_w(px(420.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(subtitle_text),
                    ),
            )
            .child(div().w_full().overflow_hidden().py(px(1.)).child(track))
            .child(footer)
            .child(pro_toast_banner(self.toast.clone()))
            .child(
                div()
                    .id(pro_eid(&self.id, "sr-caption"))
                    .role(Role::Status)
                    .aria_label(sr_announce)
                    .absolute()
                    .size_0(),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentGalleryBlock, GalleryItem, GalleryKpi, GalleryScreenTab, default_gallery_items,
    };
    use gpui::{TestAppContext, rgb};

    #[test]
    fn default_gallery_starts_on_middle_churn_watch_panel() {
        let items = default_gallery_items();
        assert_eq!(items.len(), 7);
        assert_eq!(items[items.len() / 2].id.as_ref(), "churn-watch");
        assert_eq!(items[3].kpis[0].value.as_ref(), "183");
        assert_eq!(items[3].kpis[1].value.as_ref(), "97%");
        assert_eq!(items[3].kpis[2].value.as_ref(), "3.3h");
        assert_eq!(items[3].kpis[3].value.as_ref(), "7");
    }

    #[test]
    fn gallery_navigation_and_builders_wrap_and_clamp_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let custom = vec![
            GalleryItem::new(
                "alpha",
                "Alpha",
                "First workflow",
                rgb(0x4b7bec).into(),
                rgb(0xc6d8ff).into(),
            )
            .with_kpis([
                GalleryKpi::new("RUNS", "10", vec![0.2, 0.8]),
                GalleryKpi::new("SUCCESS", "100%", vec![0.9, 1.0]),
                GalleryKpi::new("TIME SAVED", "1.0h", vec![0.3, 0.7]),
                GalleryKpi::new("ESCALATED", "0", vec![0.1, 0.1]),
            ]),
            GalleryItem::new(
                "beta",
                "Beta",
                "Second workflow",
                rgb(0x20bf6b).into(),
                rgb(0xb8f2d0).into(),
            ),
            GalleryItem::new(
                "gamma",
                "Gamma",
                "Third workflow",
                rgb(0xeb3b5a).into(),
                rgb(0xfbc2cc).into(),
            ),
        ];

        let (view, w) = app.add_window_view(|_, cx| {
            AgentGalleryBlock::new("test-gallery", cx)
                .with_items(custom)
                .with_default_index(99)
        });

        w.update(|_, cx| {
            view.update(cx, |g, cx| {
                assert_eq!(g.active_index(), 2);
                assert_eq!(g.screen_tab, GalleryScreenTab::Context);
                g.select_next(cx);
                assert_eq!(g.active_index(), 0);
                g.select_prev(cx);
                assert_eq!(g.active_index(), 2);
            });
        });
    }
}
