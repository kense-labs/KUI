//! Feature-section agent gallery with slim gradient panels flanking an open product screen.
//!
//! Displays a centered carousel of slim vertical gradient panels flanking one wide open
//! agent workspace screen; clicking a side panel or pressing the arrow keys smoothly glides
//! that panel open while collapsing the previously open panel.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_card_shadow, pro_check_draw, pro_eid,
        pro_empty_state, pro_icon_btn, pro_mark_selected, pro_mix_color, pro_person_avatar,
        pro_pill_indicator, pro_rolling_number, pro_selection_sensor, pro_skeleton_bar,
        pro_swap_label, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg, pro_svg_doc},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, LazyLock, Mutex},
};
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
                    "ESCALATIONS",
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
                "ESCALATIONS",
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
                "ESCALATIONS",
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
                "ESCALATIONS",
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
                "ESCALATIONS",
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
                "ESCALATIONS",
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
                "ESCALATIONS",
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
                "ESCALATIONS",
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
        "ESCALATIONS" | "ESCALATED" => "人工升级",
        _ => label,
    }
}

const INTER_MEDIUM_ASCII: [(f32, &str); 95] = [
    (26.7, ""),
    (
        30.4,
        "M10 -23.3 9.3 -72.8H21.1L20.4 -23.3ZM15.2 0.7Q12.1 0.7 10 -1.3Q8 -3.4 8 -6.4Q8 -9.4 10 -11.5Q12.1 -13.5 15.2 -13.5Q18.3 -13.5 20.4 -11.5Q22.5 -9.4 22.5 -6.4Q22.5 -3.4 20.4 -1.3Q18.3 0.7 15.2 0.7Z",
    ),
    (
        49.4,
        "M11.3 -44.2 10.1 -72.8H21.2L19.9 -44.2ZM29.5 -44.2 28.3 -72.8H39.3L38.1 -44.2Z",
    ),
    (
        63.9,
        "M34.4 0 46.4 -72.8H55L43 0ZM1.5 -19.4 2.9 -28H58.1L56.7 -19.4ZM8.9 0 20.8 -72.8H29.4L17.5 0ZM5.7 -44.7 7.2 -53.3H62.3L60.9 -44.7Z",
    ),
    (
        64.6,
        "M29.5 10.4V-83.1H35.4V10.4ZM32.5 1.2Q24.3 1.2 18.3 -1.4Q12.3 -4 8.9 -8.8Q5.5 -13.6 5.2 -20.2H16.3Q16.7 -16.3 18.9 -13.7Q21.2 -11.1 24.7 -9.8Q28.3 -8.5 32.4 -8.5Q37 -8.5 40.6 -10Q44.3 -11.4 46.3 -14.1Q48.4 -16.8 48.4 -20.2Q48.4 -23.4 46.6 -25.5Q44.8 -27.5 41.7 -28.8Q38.6 -30.2 34.6 -31.2L26.1 -33.5Q17.2 -35.9 12.2 -40.6Q7.2 -45.3 7.2 -52.8Q7.2 -59.1 10.6 -63.8Q14 -68.5 19.8 -71.1Q25.6 -73.7 33 -73.7Q40.4 -73.7 46.1 -71.1Q51.8 -68.5 55 -64Q58.3 -59.4 58.5 -53.6H47.6Q47 -58.6 43 -61.3Q38.9 -64.1 32.7 -64.1Q28.3 -64.1 25.1 -62.7Q21.8 -61.3 20.1 -58.9Q18.3 -56.5 18.3 -53.4Q18.3 -50 20.4 -47.8Q22.5 -45.7 25.6 -44.5Q28.6 -43.2 31.4 -42.5L38.5 -40.6Q41.9 -39.8 45.6 -38.3Q49.3 -36.8 52.4 -34.4Q55.6 -32 57.5 -28.5Q59.4 -25 59.4 -20.1Q59.4 -13.9 56.3 -9.1Q53.1 -4.3 47.1 -1.5Q41 1.2 32.5 1.2Z",
    ),
    (
        99.3,
        "M25.3 -39.7Q20.5 -39.7 17.2 -41.8Q13.9 -43.9 12.2 -47.4Q10.5 -50.8 10.5 -54.9V-58.7Q10.5 -62.9 12.2 -66.3Q14 -69.8 17.3 -71.8Q20.6 -73.9 25.3 -73.9Q30.2 -73.9 33.5 -71.8Q36.7 -69.8 38.4 -66.3Q40.1 -62.9 40.1 -58.7V-54.9Q40.1 -50.8 38.4 -47.3Q36.7 -43.9 33.4 -41.8Q30.1 -39.7 25.3 -39.7ZM25.3 -47Q28.9 -47 30.3 -49.4Q31.7 -51.9 31.7 -54.9V-58.7Q31.7 -61.8 30.4 -64.3Q29 -66.7 25.3 -66.7Q21.8 -66.7 20.4 -64.2Q18.9 -61.7 18.9 -58.7V-54.9Q18.9 -51.9 20.3 -49.4Q21.8 -47 25.3 -47ZM74 1.4Q69.2 1.4 65.9 -0.7Q62.6 -2.8 60.9 -6.3Q59.2 -9.7 59.2 -13.8V-17.6Q59.2 -21.8 60.9 -25.2Q62.7 -28.7 66 -30.7Q69.3 -32.8 74 -32.8Q78.9 -32.8 82.2 -30.7Q85.5 -28.7 87.1 -25.2Q88.8 -21.8 88.8 -17.6V-13.8Q88.8 -9.7 87.1 -6.2Q85.4 -2.8 82.1 -0.7Q78.8 1.4 74 1.4ZM74 -5.9Q77.6 -5.9 79 -8.3Q80.4 -10.8 80.4 -13.8V-17.6Q80.4 -20.7 79.1 -23.2Q77.7 -25.6 74 -25.6Q70.5 -25.6 69.1 -23.1Q67.6 -20.6 67.6 -17.6V-13.8Q67.6 -10.8 69 -8.3Q70.5 -5.9 74 -5.9ZM20 0 70 -72.8H78.8L28.8 0Z",
    ),
    (
        65.3,
        "M28.3 1Q21 1 15.7 -1.7Q10.4 -4.4 7.6 -8.9Q4.7 -13.4 4.7 -19.1Q4.7 -23.7 6.6 -27.1Q8.5 -30.6 11.8 -33.5Q15.1 -36.4 19.4 -39.5L31.2 -47.8Q32.6 -48.8 34 -50.1Q35.5 -51.5 36.4 -53.2Q37.4 -55 37.4 -57.2Q37.4 -60.4 35.3 -62.4Q33.1 -64.5 29.4 -64.5Q26.9 -64.5 24.9 -63.5Q23 -62.5 21.9 -60.6Q20.9 -58.8 20.9 -56.4Q20.9 -53.8 22.1 -51.3Q23.3 -48.9 25.5 -46.2Q27.6 -43.6 30.3 -40.3L64.2 0H51.9L24 -32.8Q20.4 -37.1 17.4 -40.7Q14.4 -44.4 12.6 -48.2Q10.8 -51.9 10.8 -56.4Q10.8 -61.4 13 -65.2Q15.3 -69.1 19.5 -71.3Q23.7 -73.5 29.2 -73.5Q34.8 -73.5 38.8 -71.3Q42.9 -69.1 45.1 -65.5Q47.3 -61.8 47.3 -57.3Q47.3 -54 46 -51.1Q44.8 -48.2 42.6 -45.8Q40.5 -43.4 37.6 -41.2L23 -30.5Q19 -27.5 17.2 -24.9Q15.5 -22.3 15.5 -19.1Q15.5 -15.8 17.1 -13.3Q18.6 -10.8 21.5 -9.4Q24.3 -8 28 -8Q32.3 -8 36.2 -9.8Q40.1 -11.6 43.3 -14.9Q46.4 -18.2 48.3 -22.8Q50.1 -27.4 50.1 -33.2H59.8Q59.8 -26.1 58.2 -21.2Q56.6 -16.4 54.4 -13.3Q52.1 -10.2 50.1 -8.5L47.7 -6.2Q44 -2.6 38.7 -0.8Q33.4 1 28.3 1Z",
    ),
    (31.3, "M11.3 -44.2 10.1 -72.8H21.2L19.9 -44.2Z"),
    (
        36.9,
        "M9.9 -29.6Q9.9 -37.6 11.5 -46.4Q13.1 -55.1 15.9 -63.2Q18.8 -71.2 22.5 -77.2H32.7Q29 -69.6 26.3 -61.3Q23.6 -53 22.2 -44.9Q20.8 -36.8 20.8 -29.6Q20.8 -23.2 22 -16.6Q23.3 -10 25.9 -2.6Q28.5 4.8 32.7 13.6H22.5Q16.3 3.1 13.1 -7.9Q9.9 -18.9 9.9 -29.6Z",
    ),
    (
        36.9,
        "M4.2 13.6Q8.5 4.6 11.1 -2.9Q13.7 -10.3 14.9 -16.8Q16.1 -23.4 16.1 -29.6Q16.1 -36.8 14.7 -44.9Q13.3 -53 10.6 -61.3Q7.9 -69.6 4.2 -77.2H14.4Q18.1 -71.2 21 -63.2Q23.8 -55.1 25.4 -46.4Q27 -37.6 27 -29.6Q27 -18.7 23.8 -7.7Q20.6 3.3 14.4 13.6Z",
    ),
    (
        52.1,
        "M22.1 -31.8 22.8 -46.8 10.2 -38.6 6.3 -45.5 19.7 -52.2 6.3 -59 10.2 -66 22.8 -57.8 22.1 -72.8H29.9L29.2 -57.8L41.8 -66L45.7 -59L32.4 -52.2L45.7 -45.5L41.8 -38.6L29.2 -46.8L29.9 -31.8Z",
    ),
    (
        66.7,
        "M28.6 -4.6V-52.7H38.2V-4.6ZM9.3 -24.1V-33.2H57.4V-24.1Z",
    ),
    (30.3, "M6.1 18.1 10.4 -10.4H22L13.9 18.1Z"),
    (46.2, "M39.3 -35.3V-26H7V-35.3Z"),
    (
        30.3,
        "M15.2 0.7Q12.2 0.7 10.1 -1.4Q8 -3.5 8 -6.5Q8 -9.5 10.1 -11.6Q12.2 -13.6 15.2 -13.6Q18.2 -13.6 20.3 -11.6Q22.4 -9.5 22.4 -6.5Q22.4 -3.5 20.3 -1.4Q18.2 0.7 15.2 0.7Z",
    ),
    (37.0, "M35 -76.2 11.6 10.9H1.9L25.4 -76.2Z"),
    (
        64.6,
        "M32.3 1Q23.8 1 17.8 -3.4Q11.8 -7.8 8.7 -16.2Q5.5 -24.5 5.5 -36.3Q5.5 -48.1 8.7 -56.5Q11.9 -64.8 17.9 -69.3Q23.8 -73.7 32.3 -73.7Q40.7 -73.7 46.7 -69.3Q52.7 -64.8 55.9 -56.5Q59 -48.1 59 -36.3Q59 -24.5 55.9 -16.2Q52.7 -7.8 46.8 -3.4Q40.8 1 32.3 1ZM32.3 -8.6Q37.3 -8.6 40.8 -11.8Q44.3 -15.1 46.2 -21.3Q48 -27.5 48 -36.3Q48 -45.2 46.2 -51.4Q44.3 -57.7 40.8 -61Q37.3 -64.3 32.3 -64.3Q27.2 -64.3 23.7 -60.9Q20.2 -57.6 18.4 -51.4Q16.5 -45.2 16.5 -36.3Q16.5 -27.5 18.4 -21.3Q20.2 -15.1 23.7 -11.8Q27.2 -8.6 32.3 -8.6Z",
    ),
    (
        41.5,
        "M33.4 -72.8V0H22.4V-62.6H21.9L4.6 -50V-61.2L20.5 -72.8Z",
    ),
    (
        61.6,
        "M7 0V-8L31.7 -33.6Q35.5 -37.6 38.1 -40.7Q40.6 -43.7 42 -46.6Q43.3 -49.4 43.3 -52.7Q43.3 -56.3 41.6 -58.9Q39.9 -61.6 37 -63Q34.1 -64.4 30.4 -64.4Q26.5 -64.4 23.6 -62.8Q20.7 -61.2 19.2 -58.3Q17.6 -55.5 17.6 -51.5H6.9Q6.9 -58.2 10 -63.2Q13.1 -68.2 18.4 -71Q23.7 -73.7 30.6 -73.7Q37.5 -73.7 42.7 -71Q48 -68.3 51 -63.6Q53.9 -58.9 53.9 -53Q53.9 -49 52.5 -45.2Q51 -41.4 47.3 -36.6Q43.5 -31.8 36.7 -25L22.5 -10.1V-9.5H55.1V0Z",
    ),
    (
        62.7,
        "M31.4 1Q24 1 18.3 -1.6Q12.6 -4.1 9.2 -8.6Q5.9 -13.1 5.7 -19H16.9Q17.2 -15.8 19.1 -13.5Q21 -11.1 24.2 -9.8Q27.4 -8.6 31.3 -8.6Q35.6 -8.6 38.8 -10.1Q42.1 -11.5 44 -14.2Q45.9 -16.9 45.9 -20.4Q45.9 -24 44 -26.8Q42.2 -29.5 38.7 -31.1Q35.1 -32.7 30.1 -32.7H23.9V-41.7H30.1Q34.1 -41.7 37.2 -43.2Q40.3 -44.6 42 -47.2Q43.7 -49.8 43.7 -53.3Q43.7 -56.6 42.2 -59.1Q40.7 -61.6 37.9 -63Q35.2 -64.4 31.5 -64.4Q28 -64.4 24.9 -63.1Q21.9 -61.8 20 -59.5Q18.1 -57.1 17.9 -53.8H7.2Q7.4 -59.7 10.7 -64.2Q14 -68.7 19.5 -71.2Q24.9 -73.7 31.7 -73.7Q38.7 -73.7 43.8 -71Q49 -68.3 51.8 -63.8Q54.6 -59.3 54.6 -53.9Q54.6 -47.8 51.1 -43.5Q47.7 -39.3 42 -37.8V-37.2Q46.8 -36.5 50.2 -34.2Q53.6 -31.8 55.4 -28.2Q57.2 -24.5 57.2 -20Q57.2 -13.9 53.9 -9.2Q50.5 -4.5 44.7 -1.7Q38.9 1 31.4 1Z",
    ),
    (
        65.6,
        "M5.5 -14.2V-23.1L36.9 -72.8H44V-59.5H39.5L17.2 -24.2V-23.6H60.3V-14.2ZM40 0V-16.9L40.1 -21V-72.8H50.7V0Z",
    ),
    (
        60.3,
        "M29.6 1Q22.9 1 17.5 -1.6Q12.2 -4.2 9.1 -8.7Q5.9 -13.3 5.7 -19H16.5Q16.8 -16 18.6 -13.5Q20.4 -11.1 23.3 -9.8Q26.2 -8.4 29.6 -8.4Q33.8 -8.4 37 -10.3Q40.2 -12.2 42.1 -15.6Q43.9 -19 43.9 -23.3Q43.9 -27.8 42 -31.2Q40.1 -34.7 36.8 -36.7Q33.5 -38.6 29.1 -38.6Q25.8 -38.6 22.5 -37.5Q19.2 -36.3 17.2 -34.5L6.9 -36L11 -72.8H51.2V-63.2H20.3L18 -42.9H18.4Q20.6 -45 24.1 -46.4Q27.7 -47.8 31.7 -47.8Q36.7 -47.8 40.9 -46Q45.1 -44.2 48.2 -40.9Q51.3 -37.7 53.1 -33.2Q54.8 -28.8 54.8 -23.6Q54.8 -16.5 51.6 -10.9Q48.3 -5.4 42.7 -2.2Q37 1 29.6 1Z",
    ),
    (
        63.0,
        "M32.2 1Q27.3 1 22.5 -0.7Q17.8 -2.5 13.9 -6.5Q10.1 -10.4 7.8 -17.2Q5.5 -23.9 5.5 -33.9Q5.5 -43.4 7.4 -50.8Q9.3 -58.2 12.9 -63.3Q16.5 -68.4 21.6 -71.1Q26.7 -73.7 33.1 -73.7Q39.6 -73.7 44.6 -71.2Q49.7 -68.6 52.9 -64.1Q56 -59.6 56.9 -53.8H45.9Q44.8 -58.3 41.6 -61.1Q38.3 -64 33.1 -64Q27.8 -64 24 -60.9Q20.2 -57.7 18.2 -51.9Q16.2 -46 16.2 -37.8H16.8Q18.6 -40.9 21.4 -43.1Q24.1 -45.3 27.6 -46.5Q31 -47.6 34.8 -47.6Q41.1 -47.6 46.3 -44.6Q51.4 -41.5 54.5 -36.1Q57.5 -30.7 57.5 -23.8Q57.5 -16.8 54.3 -11.2Q51.2 -5.6 45.5 -2.3Q39.8 1 32.2 1ZM32.1 -8.4Q36.3 -8.4 39.6 -10.4Q42.8 -12.5 44.8 -15.9Q46.7 -19.4 46.7 -23.6Q46.7 -27.8 44.8 -31.2Q43 -34.6 39.7 -36.6Q36.5 -38.6 32.3 -38.6Q29.2 -38.6 26.5 -37.4Q23.9 -36.2 21.9 -34.1Q19.9 -32 18.7 -29.3Q17.6 -26.6 17.6 -23.6Q17.6 -19.5 19.5 -16.1Q21.4 -12.6 24.7 -10.5Q28 -8.4 32.1 -8.4Z",
    ),
    (57.1, "M9.4 0 41 -62.7V-63.2H4.4V-72.8H52.7V-62.8L21.1 0Z"),
    (
        62.9,
        "M31.4 1Q23.8 1 18 -1.7Q12.1 -4.3 8.8 -8.9Q5.5 -13.5 5.5 -19.4Q5.5 -24 7.4 -27.9Q9.4 -31.8 12.7 -34.4Q16 -37 20.2 -37.7V-38.1Q14.7 -39.5 11.4 -44Q8.1 -48.5 8.1 -54.3Q8.1 -59.9 11.1 -64.3Q14.2 -68.7 19.4 -71.2Q24.7 -73.7 31.4 -73.7Q38.1 -73.7 43.4 -71.2Q48.7 -68.7 51.7 -64.3Q54.7 -59.9 54.7 -54.3Q54.7 -48.5 51.4 -44Q48.1 -39.5 42.8 -38.1V-37.7Q46.8 -37 50.2 -34.4Q53.5 -31.8 55.5 -27.9Q57.4 -24 57.4 -19.4Q57.4 -13.5 54.1 -8.9Q50.8 -4.3 44.9 -1.7Q39 1 31.4 1ZM31.4 -8.1Q35.9 -8.1 39.2 -9.6Q42.5 -11.1 44.3 -13.8Q46.1 -16.5 46.1 -20.1Q46.1 -23.9 44.2 -26.8Q42.3 -29.7 39 -31.3Q35.6 -33 31.4 -33Q27.2 -33 23.9 -31.3Q20.6 -29.7 18.7 -26.8Q16.7 -23.9 16.7 -20.1Q16.7 -16.5 18.5 -13.8Q20.3 -11.1 23.6 -9.6Q26.9 -8.1 31.4 -8.1ZM31.4 -41.8Q35.1 -41.8 37.8 -43.3Q40.6 -44.7 42.3 -47.3Q43.9 -49.9 43.9 -53.3Q43.9 -56.8 42.3 -59.3Q40.8 -61.8 37.9 -63.2Q35.1 -64.6 31.4 -64.6Q27.7 -64.6 24.9 -63.2Q22.1 -61.8 20.5 -59.3Q19 -56.8 19 -53.3Q19 -49.9 20.6 -47.3Q22.1 -44.7 25 -43.3Q27.8 -41.8 31.4 -41.8Z",
    ),
    (
        63.0,
        "M30 1Q23.5 1 18.4 -1.6Q13.3 -4.1 10.1 -8.7Q7 -13.2 6.1 -19.1H17.2Q18.2 -14.6 21.5 -11.7Q24.7 -8.7 30 -8.7Q35.3 -8.7 39.1 -11.9Q42.8 -15 44.8 -20.9Q46.8 -26.8 46.8 -35H46.3Q44.4 -32 41.7 -29.8Q38.9 -27.6 35.5 -26.4Q32.1 -25.1 28.3 -25.1Q21.9 -25.1 16.8 -28.2Q11.6 -31.3 8.5 -36.7Q5.5 -42.1 5.5 -49Q5.5 -56 8.7 -61.7Q11.9 -67.3 17.6 -70.6Q23.3 -73.9 31 -73.8Q35.8 -73.7 40.5 -72Q45.2 -70.2 49.1 -66.3Q52.9 -62.3 55.2 -55.6Q57.5 -48.9 57.5 -38.9Q57.5 -29.4 55.6 -22Q53.7 -14.6 50.1 -9.4Q46.5 -4.3 41.4 -1.6Q36.3 1 30 1ZM30.7 -34.2Q33.8 -34.2 36.5 -35.4Q39.2 -36.6 41.2 -38.8Q43.2 -40.9 44.3 -43.6Q45.5 -46.3 45.5 -49.4Q45.5 -53.3 43.6 -56.7Q41.7 -60.1 38.4 -62.3Q35.1 -64.4 30.9 -64.4Q26.8 -64.4 23.5 -62.3Q20.2 -60.3 18.3 -56.9Q16.4 -53.4 16.4 -49.2Q16.4 -45 18.3 -41.6Q20.1 -38.2 23.3 -36.2Q26.5 -34.2 30.7 -34.2Z",
    ),
    (
        30.3,
        "M15.2 0.7Q12.2 0.7 10.1 -1.4Q8 -3.5 8 -6.5Q8 -9.5 10.1 -11.6Q12.2 -13.6 15.2 -13.6Q18.2 -13.6 20.3 -11.6Q22.4 -9.5 22.4 -6.5Q22.4 -3.5 20.3 -1.4Q18.2 0.7 15.2 0.7ZM15.2 -37.7Q12.2 -37.7 10.1 -39.8Q8 -41.9 8 -44.9Q8 -47.9 10.1 -50Q12.2 -52.1 15.2 -52.1Q18.2 -52.1 20.3 -50Q22.4 -47.9 22.4 -44.9Q22.4 -41.9 20.3 -39.8Q18.2 -37.7 15.2 -37.7Z",
    ),
    (
        31.5,
        "M6.1 18.1 10.4 -10.4H22L13.9 18.1ZM16.4 -37.7Q13.4 -37.7 11.3 -39.8Q9.2 -41.9 9.2 -44.9Q9.2 -47.9 11.3 -50Q13.4 -52.1 16.4 -52.1Q19.4 -52.1 21.5 -50Q23.6 -47.9 23.6 -44.9Q23.6 -41.9 21.5 -39.8Q19.4 -37.7 16.4 -37.7Z",
    ),
    (
        66.7,
        "M8.4 -24.6V-32.7L55.7 -55.6V-44.5L20.9 -28.8L21.2 -29.4V-27.9L20.9 -28.5L55.7 -12.7V-1.7Z",
    ),
    (
        66.7,
        "M10.6 -34.9V-44.4H56.2V-34.9ZM10.6 -12.9V-22.5H56.2V-12.9Z",
    ),
    (
        66.7,
        "M58.4 -24.6 11.1 -1.7V-12.7L45.9 -28.5L45.5 -27.9V-29.4L45.9 -28.8L11.1 -44.5V-55.6L58.4 -32.7Z",
    ),
    (
        52.7,
        "M18.8 -22.1V-22.8Q18.8 -29.4 20 -33Q21.3 -36.6 23.6 -38.7Q25.9 -40.8 29.2 -42.8Q32.4 -44.8 34.6 -47.6Q36.8 -50.4 36.8 -54.5Q36.8 -57.6 35.4 -59.8Q33.9 -62.1 31.5 -63.4Q29 -64.6 26 -64.6Q23.3 -64.6 20.8 -63.5Q18.3 -62.3 16.7 -60Q15.1 -57.6 14.8 -53.9H4Q4.2 -60.4 7.2 -64.8Q10.2 -69.2 15.1 -71.5Q20 -73.7 26 -73.7Q32.6 -73.7 37.5 -71.3Q42.4 -68.9 45.1 -64.5Q47.8 -60.2 47.8 -54.4Q47.8 -48.4 45.1 -44.2Q42.4 -40 37.6 -37.1Q34.6 -35.3 32.7 -33.5Q30.8 -31.8 29.9 -29.3Q29.1 -26.9 29.1 -22.8V-22.1ZM24 0.7Q20.9 0.7 18.9 -1.3Q16.8 -3.4 16.8 -6.4Q16.8 -9.4 18.9 -11.5Q20.9 -13.5 24 -13.5Q27.1 -13.5 29.2 -11.5Q31.3 -9.4 31.3 -6.4Q31.3 -3.4 29.2 -1.3Q27.1 0.7 24 0.7Z",
    ),
    (
        98.2,
        "M51.3 19.8Q40.3 19.8 31.7 16.8Q23.2 13.8 17.4 8Q11.5 2.2 8.5 -6.2Q5.5 -14.7 5.5 -25.5Q5.5 -35.9 8.5 -44.3Q11.5 -52.7 17.3 -58.6Q23.1 -64.6 31.4 -67.7Q39.8 -70.9 50.5 -70.9Q60.7 -70.9 68.6 -67.7Q76.4 -64.5 81.8 -58.9Q87.2 -53.3 90 -46.1Q92.8 -38.9 92.8 -30.9Q92.8 -25.4 92 -20.2Q91.3 -14.9 89.4 -10.7Q87.4 -6.4 83.9 -3.9Q80.4 -1.4 74.8 -1.4Q72.1 -1.4 69.1 -2.2Q66.2 -3 64.1 -4.8Q61.9 -6.6 61.5 -9.6H61.1Q59.8 -6.3 56.3 -4Q52.9 -1.6 47.3 -1.6Q40.7 -1.6 36.2 -4.8Q31.8 -8 29.5 -13.5Q27.2 -19 27.2 -26Q27.2 -32.7 29.8 -38Q32.4 -43.3 37.1 -46.3Q41.7 -49.4 47.7 -49.4Q52.4 -49.4 55.7 -47.7Q58.9 -45.9 60.3 -43.8H60.8V-48.2H69.3V-15.8Q69.3 -13 70.7 -11.1Q72.1 -9.2 75 -9.2Q78.4 -9.2 80.3 -11.5Q82.1 -13.8 82.9 -18.5Q83.6 -23.3 83.6 -30.7Q83.6 -37.1 81.6 -42.8Q79.6 -48.6 75.5 -53Q71.4 -57.4 65.1 -60Q58.9 -62.5 50.6 -62.5Q42.1 -62.5 35.5 -60Q28.9 -57.5 24.3 -52.7Q19.8 -47.9 17.4 -41Q15.1 -34.1 15.1 -25.4Q15.1 -16.6 17.4 -9.7Q19.8 -2.9 24.5 1.8Q29.2 6.5 36 8.9Q42.9 11.4 51.9 11.4Q55.8 11.4 59.5 10.7Q63.2 10 66.2 9.1Q69.2 8.2 70.8 7.6L73.5 15.5Q71.2 16.5 67.5 17.5Q63.9 18.5 59.7 19.2Q55.4 19.8 51.3 19.8ZM48 -10.1Q52.5 -10.1 55.3 -11.9Q58.1 -13.7 59.4 -17.3Q60.7 -20.9 60.7 -26.3Q60.7 -31.8 59.3 -34.9Q57.9 -38.1 55.1 -39.4Q52.3 -40.8 48.2 -40.8Q44.3 -40.8 41.5 -38.8Q38.7 -36.9 37.3 -33.6Q35.8 -30.3 35.8 -26.3Q35.8 -22 37 -18.2Q38.3 -14.5 40.9 -12.3Q43.6 -10.1 48 -10.1Z",
    ),
    (
        70.9,
        "M2.5 0 28.4 -72.8H42.1L68.4 0H56.4L42.2 -40.5Q40.5 -45.4 38.6 -52.1Q36.6 -58.8 34 -68.1H36.3Q33.7 -58.7 31.8 -51.9Q29.8 -45.2 28.3 -40.5L14.5 0ZM16.5 -19.2V-28.5H54.4V-19.2Z",
    ),
    (
        65.7,
        "M8.1 0V-72.8H34.9Q42.6 -72.8 47.7 -70.2Q52.8 -67.7 55.3 -63.3Q57.8 -59 57.8 -53.7Q57.8 -49.1 56.1 -46Q54.4 -42.9 51.6 -41Q48.8 -39.1 45.5 -38.3V-37.6Q49.1 -37.4 52.6 -35.2Q56.1 -33.1 58.3 -29.1Q60.6 -25.2 60.6 -19.6Q60.6 -14 58 -9.6Q55.4 -5.2 49.9 -2.6Q44.5 0 36 0ZM19.2 -9.6H34.9Q42.7 -9.6 46.1 -12.6Q49.5 -15.6 49.5 -20.1Q49.5 -23.6 47.8 -26.5Q46.1 -29.3 42.9 -31Q39.7 -32.7 35.3 -32.7H19.2ZM19.2 -41.3H33.8Q37.5 -41.3 40.4 -42.7Q43.4 -44.1 45.1 -46.7Q46.8 -49.2 46.8 -52.7Q46.8 -57.2 43.7 -60.2Q40.6 -63.2 34.1 -63.2H19.2Z",
    ),
    (
        73.3,
        "M38.3 1Q28.8 1 21.4 -3.5Q14 -8 9.8 -16.3Q5.5 -24.7 5.5 -36.3Q5.5 -48 9.8 -56.4Q14 -64.7 21.5 -69.2Q28.9 -73.7 38.3 -73.7Q44 -73.7 49.1 -72.1Q54.1 -70.4 58.1 -67.3Q62.1 -64.1 64.7 -59.5Q67.4 -55 68.3 -49.2H57.1Q56.4 -52.7 54.7 -55.4Q53 -58 50.4 -59.9Q47.9 -61.7 44.9 -62.7Q41.8 -63.6 38.4 -63.6Q32.1 -63.6 27.2 -60.4Q22.2 -57.3 19.4 -51.2Q16.6 -45.1 16.6 -36.3Q16.6 -27.5 19.4 -21.5Q22.3 -15.4 27.2 -12.3Q32.2 -9.2 38.4 -9.2Q41.8 -9.2 44.8 -10.1Q47.9 -11.1 50.4 -12.9Q52.9 -14.7 54.7 -17.4Q56.4 -20.1 57.1 -23.6H68.3Q67.5 -18.2 64.9 -13.7Q62.4 -9.2 58.4 -5.9Q54.5 -2.6 49.4 -0.8Q44.3 1 38.3 1Z",
    ),
    (
        72.2,
        "M31.7 0H13.7V-9.7H31.1Q39.4 -9.7 44.8 -12.8Q50.3 -15.9 53 -21.9Q55.8 -27.9 55.8 -36.5Q55.8 -45 53 -50.9Q50.3 -56.9 45 -59.9Q39.7 -63 31.7 -63H13.3V-72.8H32.4Q43.2 -72.8 50.8 -68.4Q58.5 -64.1 62.6 -55.9Q66.7 -47.8 66.7 -36.5Q66.7 -25.1 62.6 -16.9Q58.4 -8.8 50.6 -4.4Q42.8 0 31.7 0ZM19.2 -72.8V0H8.1V-72.8Z",
    ),
    (
        60.3,
        "M8.1 0V-72.8H53.8V-63.2H19.2V-41.5H51.4V-32H19.2V-9.6H54.2V0Z",
    ),
    (
        58.9,
        "M8.1 0V-72.8H53.3V-63.2H19.2V-39.8H50.1V-30.3H19.2V0Z",
    ),
    (
        74.8,
        "M38.6 1Q28.7 1 21.3 -3.6Q13.8 -8.1 9.7 -16.5Q5.5 -24.8 5.5 -36.3Q5.5 -48 9.7 -56.3Q14 -64.7 21.3 -69.2Q28.7 -73.7 38.1 -73.7Q44 -73.7 49.1 -72Q54.2 -70.3 58.2 -67.1Q62.2 -63.9 64.9 -59.4Q67.5 -55 68.4 -49.7H56.9Q55.9 -52.9 54.2 -55.5Q52.5 -58 50.2 -59.9Q47.8 -61.7 44.8 -62.6Q41.8 -63.6 38.2 -63.6Q32.1 -63.6 27.2 -60.5Q22.3 -57.3 19.4 -51.3Q16.6 -45.2 16.6 -36.4Q16.6 -27.6 19.4 -21.5Q22.3 -15.5 27.3 -12.3Q32.2 -9.2 38.7 -9.2Q44.5 -9.2 48.9 -11.5Q53.3 -13.9 55.7 -18.3Q58.1 -22.7 58.1 -28.6L61 -28.1H40.3V-37.3H69V-29Q69 -19.8 65.1 -13Q61.1 -6.3 54.3 -2.7Q47.5 1 38.6 1Z",
    ),
    (
        74.5,
        "M8.1 0V-72.8H19.2V-42H55.2V-72.8H66.4V0H55.2V-32.4H19.2V0Z",
    ),
    (27.2, "M19.2 -72.8V0H8.1V-72.8Z"),
    (
        57.5,
        "M27 1Q16.8 1 10.6 -4.8Q4.4 -10.5 4.4 -21.2V-25.2H15.5V-21.1Q15.5 -14.9 18.6 -11.7Q21.8 -8.5 27 -8.5Q32.1 -8.5 35.3 -11.7Q38.4 -14.9 38.4 -21.1V-72.8H49.5V-21.2Q49.5 -10.5 43.3 -4.8Q37.1 1 27 1Z",
    ),
    (
        68.8,
        "M16 -19.4V-30.5Q18.4 -34.1 20.7 -37.3Q23.1 -40.6 25.7 -43.7Q28.3 -46.8 31.2 -50.1L51.6 -72.8H65.8L34.8 -38.7L34.1 -38.9ZM8.1 0V-72.8H19.2V-49.5L19 -32.9L19.2 -27.4V0ZM52.9 0 28.1 -35.6 35.1 -44.1 66.1 0Z",
    ),
    (56.5, "M8.1 0V-72.8H19.2V-9.6H52.1V0Z"),
    (
        91.3,
        "M8.1 0V-72.8H24.5L40.3 -31Q41 -29.2 41.9 -26.3Q42.7 -23.5 43.6 -20.1Q44.6 -16.8 45.4 -13.6Q46.3 -10.4 46.9 -7.8H44.5Q45.2 -10.3 46 -13.4Q46.8 -16.6 47.8 -20Q48.7 -23.3 49.6 -26.2Q50.4 -29.1 51.1 -31L66.8 -72.8H83.2V0H72.2V-39.1Q72.2 -41.4 72.2 -44.5Q72.2 -47.6 72.3 -51.1Q72.4 -54.6 72.5 -58.2Q72.6 -61.8 72.6 -65.1H73.5Q72.5 -61.5 71.3 -57.7Q70.2 -53.9 69.1 -50.4Q68 -46.9 67 -44Q66 -41.1 65.3 -39.1L50.4 0H40.8L25.8 -39.1Q25.1 -41 24.1 -43.9Q23.2 -46.7 22 -50.2Q20.9 -53.7 19.7 -57.5Q18.6 -61.3 17.4 -65.1H18.4Q18.5 -62.1 18.5 -58.6Q18.6 -55.1 18.7 -51.5Q18.8 -48 18.8 -44.8Q18.9 -41.5 18.9 -39.1V0Z",
    ),
    (
        75.6,
        "M8.1 0V-72.8H20.9L48.3 -29.5Q49.6 -27.3 51.3 -24.3Q53.1 -21.3 55 -17.6Q56.9 -13.9 58.7 -9.5H57.4Q57 -13.6 56.7 -17.7Q56.5 -21.8 56.4 -25.3Q56.3 -28.9 56.3 -31.4V-72.8H67.6V0H54.7L30.1 -38.7Q28.2 -41.9 26.3 -45.1Q24.4 -48.3 22.1 -52.5Q19.9 -56.8 16.6 -62.8H18.4Q18.6 -57.5 18.8 -52.8Q19.1 -48.1 19.2 -44.5Q19.4 -41 19.4 -38.8V0Z",
    ),
    (
        76.7,
        "M38.4 1Q28.9 1 21.5 -3.5Q14.1 -8 9.8 -16.3Q5.5 -24.7 5.5 -36.3Q5.5 -48 9.8 -56.4Q14.1 -64.7 21.5 -69.2Q28.9 -73.7 38.4 -73.7Q47.8 -73.7 55.2 -69.2Q62.6 -64.7 66.9 -56.4Q71.2 -48 71.2 -36.3Q71.2 -24.7 66.9 -16.3Q62.6 -8 55.2 -3.5Q47.8 1 38.4 1ZM38.4 -9.2Q44.6 -9.2 49.5 -12.3Q54.4 -15.4 57.2 -21.5Q60.1 -27.6 60.1 -36.3Q60.1 -45.2 57.2 -51.3Q54.4 -57.3 49.5 -60.5Q44.6 -63.6 38.4 -63.6Q32.1 -63.6 27.2 -60.4Q22.3 -57.3 19.4 -51.2Q16.6 -45.2 16.6 -36.3Q16.6 -27.6 19.4 -21.5Q22.3 -15.4 27.2 -12.3Q32.1 -9.2 38.4 -9.2Z",
    ),
    (
        64.2,
        "M8.1 0V-72.8H34.1Q42.5 -72.8 48.1 -69.6Q53.7 -66.5 56.5 -61.2Q59.3 -55.9 59.3 -49.1Q59.3 -42.4 56.5 -37.1Q53.7 -31.7 48.1 -28.6Q42.5 -25.5 34 -25.5H16.2V-34.9H32.9Q38.3 -34.9 41.6 -36.8Q45 -38.6 46.5 -41.8Q48 -45.1 48 -49.1Q48 -53.2 46.5 -56.4Q45 -59.6 41.6 -61.4Q38.3 -63.2 32.9 -63.2H19.2V0Z",
    ),
    (
        76.9,
        "M34.8 -23.5H45.5L53.2 -13.5L56.5 -9.3L68.6 6.6H57.6L49.5 -4L46.8 -7.7ZM38.4 1Q28.9 1 21.5 -3.5Q14.1 -8 9.8 -16.3Q5.5 -24.7 5.5 -36.3Q5.5 -48 9.8 -56.4Q14.1 -64.7 21.5 -69.2Q28.9 -73.7 38.4 -73.7Q47.8 -73.7 55.2 -69.2Q62.6 -64.7 66.9 -56.4Q71.2 -48 71.2 -36.3Q71.2 -24.7 66.9 -16.3Q62.6 -8 55.2 -3.5Q47.8 1 38.4 1ZM38.4 -9.2Q44.6 -9.2 49.5 -12.3Q54.4 -15.4 57.2 -21.5Q60.1 -27.6 60.1 -36.3Q60.1 -45.2 57.2 -51.3Q54.4 -57.3 49.5 -60.5Q44.6 -63.6 38.4 -63.6Q32.1 -63.6 27.2 -60.4Q22.3 -57.3 19.4 -51.2Q16.6 -45.2 16.6 -36.3Q16.6 -27.6 19.4 -21.5Q22.3 -15.4 27.2 -12.3Q32.1 -9.2 38.4 -9.2Z",
    ),
    (
        64.8,
        "M8.1 0V-72.8H34.1Q42.5 -72.8 48.1 -69.8Q53.7 -66.9 56.5 -61.8Q59.3 -56.6 59.3 -49.9Q59.3 -43.2 56.5 -38.1Q53.7 -33.1 48.1 -30.3Q42.5 -27.5 34 -27.5H14.4V-37H32.9Q38.3 -37 41.6 -38.5Q44.9 -40.1 46.5 -43Q48 -45.8 48 -49.9Q48 -54 46.5 -57Q44.9 -60 41.6 -61.6Q38.3 -63.2 32.9 -63.2H19.2V0ZM49.4 0 31.8 -32.9H44.1L62.1 0Z",
    ),
    (
        64.6,
        "M32.5 1.2Q24.3 1.2 18.3 -1.4Q12.3 -4 8.9 -8.8Q5.5 -13.6 5.2 -20.2H16.3Q16.7 -16.3 18.9 -13.7Q21.2 -11.1 24.7 -9.8Q28.3 -8.5 32.4 -8.5Q37 -8.5 40.6 -10Q44.3 -11.4 46.3 -14.1Q48.4 -16.8 48.4 -20.2Q48.4 -23.4 46.6 -25.5Q44.8 -27.5 41.7 -28.8Q38.6 -30.2 34.6 -31.2L26.1 -33.5Q17.2 -35.9 12.2 -40.6Q7.2 -45.3 7.2 -52.8Q7.2 -59.1 10.6 -63.8Q14 -68.5 19.8 -71.1Q25.6 -73.7 33 -73.7Q40.4 -73.7 46.1 -71.1Q51.8 -68.5 55 -64Q58.3 -59.4 58.5 -53.6H47.6Q47 -58.6 43 -61.3Q38.9 -64.1 32.7 -64.1Q28.3 -64.1 25.1 -62.7Q21.8 -61.3 20.1 -58.9Q18.3 -56.5 18.3 -53.4Q18.3 -50 20.4 -47.8Q22.5 -45.7 25.6 -44.5Q28.6 -43.2 31.4 -42.5L38.5 -40.6Q41.9 -39.8 45.6 -38.3Q49.3 -36.8 52.4 -34.4Q55.6 -32 57.5 -28.5Q59.4 -25 59.4 -20.1Q59.4 -13.9 56.3 -9.1Q53.1 -4.3 47.1 -1.5Q41 1.2 32.5 1.2Z",
    ),
    (65.3, "M4.4 -63.2V-72.8H60.9V-63.2H38.3V0H27.1V-63.2Z"),
    (
        74.0,
        "M37 1.1Q28.3 1.1 21.8 -2.2Q15.2 -5.6 11.6 -11.5Q8.1 -17.4 8.1 -25V-72.8H19.2V-25.9Q19.2 -21 21.4 -17.2Q23.6 -13.4 27.6 -11.2Q31.5 -9 37 -9Q42.5 -9 46.5 -11.2Q50.5 -13.4 52.6 -17.2Q54.8 -21 54.8 -25.9V-72.8H66V-25Q66 -17.4 62.4 -11.5Q58.8 -5.6 52.3 -2.2Q45.8 1.1 37 1.1Z",
    ),
    (
        70.9,
        "M28.8 0 2.5 -72.8H14.5L28.7 -32.3Q30.2 -27.6 32.3 -20.8Q34.3 -14 36.8 -4.8H34.6Q37.1 -14.2 39.1 -20.9Q41.1 -27.6 42.6 -32.3L56.3 -72.8H68.4L42.4 0Z",
    ),
    (
        100.3,
        "M22.2 0 2.5 -72.8H14.2L24.7 -31.1Q25.7 -27.2 26.5 -23Q27.4 -18.9 28.2 -14.6Q29 -10.4 29.7 -6.2H28Q28.8 -10.4 29.6 -14.6Q30.4 -18.9 31.3 -23Q32.2 -27.2 33.2 -31.1L44 -72.8H56.2L67 -31.1Q68 -27.2 68.9 -23Q69.7 -18.9 70.6 -14.6Q71.4 -10.4 72.2 -6.2H70.5Q71.2 -10.4 72 -14.6Q72.8 -18.9 73.7 -23Q74.6 -27.2 75.5 -31.1L86 -72.8H97.8L78 0H65.1L53.6 -43.1Q52.2 -48.2 51.1 -54.1Q50 -60.1 48.8 -67.2H51.4Q50.1 -60.4 49.1 -54.6Q48.1 -48.8 46.6 -43.1L35.1 0Z",
    ),
    (
        70.1,
        "M2.7 0 31.7 -41.4V-33L4.5 -72.8H17.5L27.1 -58.4Q29.4 -55 30.9 -52.5Q32.4 -50 33.6 -47.6Q34.8 -45.3 36.4 -42.3H34Q35.5 -45.3 36.7 -47.6Q38 -50 39.5 -52.5Q41 -55 43.3 -58.4L53.1 -72.8H65.8L38.8 -33.5V-41.7L67.4 0H54.4L42.5 -17.4Q40.5 -20.4 39.1 -22.6Q37.8 -24.8 36.6 -26.8Q35.5 -28.9 34 -31.8H36.1Q34.7 -29.1 33.6 -27Q32.4 -24.9 31 -22.7Q29.6 -20.5 27.5 -17.4L15.5 0Z",
    ),
    (
        69.6,
        "M29.3 0V-29L2.5 -72.8H15.5L29.2 -49.5Q31.4 -45.8 33.2 -42.3Q34.9 -38.7 36.8 -33.5H33.2Q35 -38.8 36.8 -42.4Q38.5 -45.9 40.7 -49.5L54.2 -72.8H67.1L40.4 -29V0Z",
    ),
    (
        64.1,
        "M6.1 0V-7.3L37.6 -53.5Q40 -56.9 42.7 -60.4Q45.5 -63.9 48.3 -67.3L49.3 -63.7Q44.7 -63.3 40 -63.2Q35.4 -63.2 30.7 -63.2H5.8V-72.8H58V-65.3L27 -20Q24.4 -16.3 21.6 -12.7Q18.7 -9 15.8 -5.5L14.7 -9.1Q19.4 -9.4 24.1 -9.5Q28.8 -9.6 33.5 -9.6H58.3V0Z",
    ),
    (36.9, "M10.3 13.6V-77.2H31.2V-68.5H20.6V4.9H31.2V13.6Z"),
    (37.0, "M25.4 10.9 1.9 -76.2H11.6L35 10.9Z"),
    (36.9, "M5.7 13.6V4.9H16.3V-68.5H5.7V-77.2H26.6V13.6Z"),
    (
        47.7,
        "M22.5 -65.9H25.1V-62.2H22.5ZM3.8 -39.7 18.6 -70.7H29.1L43.8 -39.7H34.3L23.5 -63.6H24.2L13.4 -39.7Z",
    ),
    (46.3, "M46.3 0V9.2H0V0Z"),
    (33.7, "M17.1 -61.6 7.5 -76.3H18.6L25.4 -61.6Z"),
    (
        56.8,
        "M22.5 1.2Q17.3 1.2 13.1 -0.7Q8.9 -2.6 6.5 -6.3Q4 -10.1 4 -15.4Q4 -20.1 5.8 -23.1Q7.6 -26 10.7 -27.8Q13.7 -29.5 17.4 -30.4Q21.2 -31.3 25.1 -31.7Q29.9 -32.3 32.9 -32.7Q35.9 -33.1 37.4 -33.9Q38.8 -34.8 38.8 -36.7V-37Q38.8 -40 37.6 -42.1Q36.5 -44.3 34.1 -45.4Q31.8 -46.5 28.3 -46.5Q24.8 -46.5 22.3 -45.4Q19.7 -44.3 18.1 -42.6Q16.5 -40.9 15.7 -39.1L5.6 -41.7Q7.5 -46.6 10.9 -49.6Q14.4 -52.5 18.9 -53.9Q23.4 -55.3 28.2 -55.3Q31.5 -55.3 35.2 -54.5Q38.9 -53.7 42.2 -51.7Q45.4 -49.6 47.5 -45.9Q49.6 -42.2 49.6 -36.2V0H39.2V-7.5H38.6Q37.6 -5.4 35.5 -3.4Q33.4 -1.4 30.2 -0.1Q26.9 1.2 22.5 1.2ZM24.9 -7.4Q29.3 -7.4 32.4 -9.1Q35.5 -10.9 37.2 -13.7Q38.9 -16.5 38.9 -19.7V-26.7Q38.3 -26.1 36.6 -25.6Q35 -25.2 32.9 -24.8Q30.8 -24.4 28.9 -24.2Q26.9 -23.9 25.6 -23.8Q22.6 -23.4 20.1 -22.4Q17.6 -21.5 16.1 -19.8Q14.6 -18.1 14.6 -15.3Q14.6 -12.7 15.9 -10.9Q17.3 -9.2 19.6 -8.3Q21.9 -7.4 24.9 -7.4Z",
    ),
    (
        61.8,
        "M34.3 1.1Q29.4 1.1 26.2 -0.6Q23.1 -2.3 21.3 -4.5Q19.5 -6.8 18.6 -8.5H17.7V0H7.2V-72.8H18V-45.7H18.6Q19.5 -47.4 21.3 -49.6Q23 -51.9 26.1 -53.6Q29.2 -55.3 34.3 -55.3Q40.9 -55.3 46 -52Q51.2 -48.6 54.1 -42.3Q57.1 -36 57.1 -27.2Q57.1 -18.3 54.2 -12Q51.3 -5.7 46.1 -2.3Q41 1.1 34.3 1.1ZM31.9 -8.1Q36.6 -8.1 39.8 -10.6Q43 -13.1 44.6 -17.5Q46.2 -21.8 46.2 -27.2Q46.2 -32.6 44.6 -36.9Q43 -41.2 39.8 -43.6Q36.7 -46.1 31.9 -46.1Q27.3 -46.1 24.2 -43.8Q21 -41.4 19.4 -37.2Q17.8 -32.9 17.8 -27.2Q17.8 -21.6 19.4 -17.2Q21.1 -12.9 24.2 -10.5Q27.4 -8.1 31.9 -8.1Z",
    ),
    (
        57.7,
        "M30.2 1.1Q22.5 1.1 16.8 -2.4Q11 -5.9 7.9 -12.2Q4.7 -18.6 4.7 -27Q4.7 -35.5 7.9 -41.9Q11 -48.2 16.8 -51.8Q22.5 -55.3 30.2 -55.3Q34.4 -55.3 38.1 -54.2Q41.8 -53.1 44.8 -51Q47.8 -48.9 49.9 -45.9Q52 -42.9 53 -39.2L42.8 -36.7Q42.3 -38.8 41.2 -40.6Q40.1 -42.3 38.5 -43.6Q36.9 -44.8 34.8 -45.5Q32.7 -46.2 30.2 -46.2Q25.3 -46.2 22 -43.6Q18.8 -41 17.2 -36.6Q15.6 -32.3 15.6 -27Q15.6 -21.8 17.2 -17.5Q18.8 -13.1 22 -10.5Q25.3 -7.9 30.2 -7.9Q32.8 -7.9 34.9 -8.7Q37 -9.4 38.6 -10.7Q40.3 -12 41.4 -13.8Q42.5 -15.6 43 -17.8L53.2 -15.4Q52.2 -11.5 50.1 -8.5Q48 -5.4 45 -3.3Q42 -1.1 38.2 0Q34.5 1.1 30.2 1.1Z",
    ),
    (
        61.8,
        "M27.5 1.1Q20.9 1.1 15.7 -2.3Q10.6 -5.7 7.6 -12Q4.7 -18.3 4.7 -27.2Q4.7 -36 7.7 -42.3Q10.7 -48.6 15.8 -52Q21 -55.3 27.5 -55.3Q32.6 -55.3 35.7 -53.6Q38.8 -51.9 40.6 -49.6Q42.3 -47.4 43.3 -45.7H43.9V-72.8H54.6V0H44.1V-8.5H43.3Q42.3 -6.8 40.5 -4.5Q38.7 -2.3 35.6 -0.6Q32.5 1.1 27.5 1.1ZM29.9 -8.1Q34.5 -8.1 37.6 -10.5Q40.8 -12.9 42.4 -17.2Q44 -21.6 44 -27.2Q44 -32.9 42.4 -37.2Q40.8 -41.4 37.7 -43.8Q34.5 -46.1 29.9 -46.1Q25.2 -46.1 22 -43.6Q18.8 -41.2 17.3 -36.9Q15.7 -32.6 15.7 -27.2Q15.7 -21.8 17.3 -17.5Q18.9 -13.1 22.1 -10.6Q25.2 -8.1 29.9 -8.1Z",
    ),
    (
        58.7,
        "M30.8 1.1Q22.7 1.1 16.8 -2.3Q11 -5.8 7.9 -12.1Q4.7 -18.4 4.7 -26.9Q4.7 -35.3 7.8 -41.7Q10.9 -48.1 16.6 -51.7Q22.2 -55.3 29.8 -55.3Q34.5 -55.3 38.9 -53.7Q43.2 -52.2 46.7 -48.9Q50.2 -45.7 52.2 -40.4Q54.2 -35.2 54.2 -27.8V-24H10.7V-32.1H48.7L43.7 -29.4Q43.7 -34.4 42.1 -38.2Q40.5 -42.1 37.5 -44.2Q34.4 -46.4 29.9 -46.4Q25.3 -46.4 22.1 -44.2Q18.9 -42 17.2 -38.4Q15.5 -34.8 15.5 -30.6V-25.1Q15.5 -19.5 17.4 -15.6Q19.3 -11.7 22.8 -9.7Q26.3 -7.7 30.9 -7.7Q33.9 -7.7 36.3 -8.6Q38.8 -9.5 40.6 -11.2Q42.3 -12.9 43.3 -15.5L53.3 -13.1Q52.1 -8.9 49 -5.7Q45.8 -2.4 41.2 -0.7Q36.6 1.1 30.8 1.1Z",
    ),
    (
        37.9,
        "M34.3 -54.6V-45.9H1V-54.6ZM11.3 0V-60.8Q11.3 -65.9 13.6 -69.3Q15.9 -72.7 19.6 -74.5Q23.3 -76.2 27.6 -76.2Q30.8 -76.2 33.2 -75.6Q35.5 -75.1 36.6 -74.7L34.1 -66Q33.3 -66.2 32.2 -66.5Q31 -66.8 29.3 -66.8Q25.5 -66.8 23.8 -64.9Q22.1 -63 22.1 -59.4V0Z",
    ),
    (
        62.0,
        "M30.1 21.6Q23.7 21.6 19.1 20Q14.5 18.4 11.5 15.7Q8.4 13 6.9 9.7L15.9 5.5Q16.9 7 18.5 8.8Q20.1 10.5 22.9 11.7Q25.7 12.9 30 12.9Q36.3 12.9 40.1 10Q44 7 44 0.7V-9.9H43.1Q42.2 -8.1 40.4 -6Q38.7 -3.9 35.6 -2.4Q32.5 -0.8 27.5 -0.8Q21.1 -0.8 15.9 -3.9Q10.7 -6.9 7.7 -12.8Q4.7 -18.8 4.7 -27.5Q4.7 -36.2 7.7 -42.4Q10.7 -48.6 15.9 -52Q21.1 -55.3 27.6 -55.3Q32.7 -55.3 35.8 -53.6Q38.9 -51.9 40.7 -49.7Q42.5 -47.4 43.5 -45.7H44.2V-54.6H54.7V0.8Q54.7 7.9 51.5 12.6Q48.2 17.2 42.6 19.4Q37.1 21.6 30.1 21.6ZM30 -9.8Q34.6 -9.8 37.7 -11.9Q40.9 -14 42.5 -18Q44.1 -22 44.1 -27.6Q44.1 -33.1 42.5 -37.3Q40.9 -41.4 37.7 -43.8Q34.6 -46.1 30 -46.1Q25.2 -46.1 22.1 -43.7Q18.9 -41.2 17.3 -37Q15.7 -32.9 15.7 -27.6Q15.7 -22.3 17.3 -18.3Q18.9 -14.3 22.1 -12Q25.3 -9.8 30 -9.8Z",
    ),
    (
        60.2,
        "M18 -32.3V0H7.2V-72.8H17.9V-41.2H16.5Q18.8 -48.5 23.4 -51.9Q27.9 -55.3 34.4 -55.3Q39.9 -55.3 44.1 -52.9Q48.3 -50.6 50.6 -46Q53 -41.4 53 -34.7V0H42.2V-33.3Q42.2 -39.2 39.1 -42.6Q36 -45.9 30.7 -45.9Q27 -45.9 24.2 -44.3Q21.3 -42.7 19.6 -39.7Q18 -36.7 18 -32.3Z",
    ),
    (
        25.2,
        "M7.2 0V-54.6H18V0ZM12.6 -62.9Q9.9 -62.9 7.9 -64.8Q5.9 -66.7 5.9 -69.3Q5.9 -72 7.9 -73.8Q9.9 -75.7 12.6 -75.7Q15.4 -75.7 17.4 -73.8Q19.4 -72 19.4 -69.3Q19.4 -66.7 17.4 -64.8Q15.4 -62.9 12.6 -62.9Z",
    ),
    (
        25.2,
        "M7.2 -54.6H18V3.5Q18 9.1 16 12.8Q14 16.6 10.1 18.5Q6.2 20.4 0.5 20.4H-1.7V11.2H0.1Q3.8 11.2 5.5 9.2Q7.2 7.2 7.2 3.5ZM12.6 -62.9Q9.8 -62.9 7.8 -64.8Q5.8 -66.7 5.8 -69.3Q5.8 -72 7.8 -73.8Q9.8 -75.7 12.6 -75.7Q15.4 -75.7 17.4 -73.8Q19.4 -72 19.4 -69.3Q19.4 -66.7 17.4 -64.8Q15.4 -62.9 12.6 -62.9Z",
    ),
    (
        55.9,
        "M17.2 -18.4V-31.5H19L40.5 -54.6H53.4L28.7 -28.2H26.9ZM7.2 0V-72.8H18V0ZM41.6 0 22.2 -25.8 29.6 -33.4 54.9 0Z",
    ),
    (25.2, "M18 -72.8V0H7.2V-72.8Z"),
    (
        88.8,
        "M7.2 0V-54.6H17.5L17.7 -41.9H16.9Q18.1 -46.6 20.5 -49.6Q23 -52.6 26.2 -54Q29.5 -55.5 33.1 -55.5Q39 -55.5 42.7 -51.8Q46.3 -48.1 47.6 -41.6H46.3Q47.3 -46.1 50 -49.2Q52.6 -52.3 56.3 -53.9Q60 -55.5 64.3 -55.5Q69.2 -55.5 73.1 -53.4Q77 -51.3 79.3 -47.2Q81.6 -43 81.6 -36.8V0H70.8V-35.7Q70.8 -41.2 67.8 -43.7Q64.8 -46.1 60.6 -46.1Q57.2 -46.1 54.8 -44.7Q52.3 -43.2 51 -40.7Q49.7 -38.2 49.7 -35V0H39.1V-36.4Q39.1 -40.8 36.3 -43.5Q33.5 -46.1 29.2 -46.1Q26.2 -46.1 23.6 -44.7Q21 -43.3 19.5 -40.6Q18 -37.9 18 -33.9V0Z",
    ),
    (
        60.2,
        "M18 -32.3V0H7.2V-54.6H17.5L17.6 -41.2H16.5Q18.8 -48.5 23.4 -51.9Q27.9 -55.3 34.4 -55.3Q39.9 -55.3 44.1 -52.9Q48.3 -50.6 50.6 -46Q52.9 -41.4 52.9 -34.7V0H42.2V-33.3Q42.2 -39.2 39.1 -42.6Q36 -45.9 30.7 -45.9Q27 -45.9 24.2 -44.3Q21.3 -42.7 19.6 -39.7Q18 -36.7 18 -32.3Z",
    ),
    (
        60.4,
        "M30.2 1.1Q22.5 1.1 16.8 -2.4Q11 -5.9 7.9 -12.2Q4.7 -18.6 4.7 -27Q4.7 -35.5 7.9 -41.9Q11 -48.2 16.8 -51.8Q22.5 -55.3 30.2 -55.3Q37.9 -55.3 43.6 -51.8Q49.4 -48.2 52.5 -41.9Q55.7 -35.5 55.7 -27Q55.7 -18.6 52.5 -12.2Q49.4 -5.9 43.6 -2.4Q37.9 1.1 30.2 1.1ZM30.2 -7.9Q35.2 -7.9 38.4 -10.6Q41.7 -13.2 43.2 -17.5Q44.8 -21.8 44.8 -27Q44.8 -32.2 43.2 -36.6Q41.7 -40.9 38.4 -43.6Q35.2 -46.2 30.2 -46.2Q25.2 -46.2 22 -43.6Q18.8 -40.9 17.2 -36.6Q15.6 -32.3 15.6 -27Q15.6 -21.8 17.2 -17.5Q18.8 -13.2 22 -10.6Q25.2 -7.9 30.2 -7.9Z",
    ),
    (
        61.8,
        "M7.2 20.4V-54.6H17.7V-45.7H18.6Q19.5 -47.4 21.3 -49.6Q23 -51.9 26.1 -53.6Q29.2 -55.3 34.3 -55.3Q40.9 -55.3 46 -52Q51.2 -48.6 54.1 -42.3Q57.1 -36 57.1 -27.2Q57.1 -18.3 54.2 -12Q51.3 -5.7 46.1 -2.3Q41 1.1 34.3 1.1Q29.4 1.1 26.2 -0.6Q23.1 -2.3 21.3 -4.5Q19.5 -6.8 18.6 -8.5H18V20.4ZM31.9 -8.1Q36.6 -8.1 39.8 -10.6Q43 -13.1 44.6 -17.5Q46.2 -21.8 46.2 -27.2Q46.2 -32.6 44.6 -36.9Q43 -41.2 39.8 -43.6Q36.7 -46.1 31.9 -46.1Q27.3 -46.1 24.2 -43.8Q21 -41.4 19.4 -37.2Q17.8 -32.9 17.8 -27.2Q17.8 -21.6 19.4 -17.2Q21.1 -12.9 24.2 -10.5Q27.4 -8.1 31.9 -8.1Z",
    ),
    (
        61.8,
        "M54.6 20.4H43.9V-8.5H43.3Q42.3 -6.8 40.5 -4.5Q38.7 -2.3 35.6 -0.6Q32.5 1.1 27.5 1.1Q20.9 1.1 15.7 -2.3Q10.6 -5.7 7.6 -12Q4.7 -18.3 4.7 -27.2Q4.7 -36 7.7 -42.3Q10.7 -48.6 15.8 -52Q21 -55.3 27.5 -55.3Q32.6 -55.3 35.7 -53.6Q38.8 -51.9 40.6 -49.6Q42.3 -47.4 43.3 -45.7H44.1V-54.6H54.6ZM29.9 -8.1Q34.5 -8.1 37.6 -10.5Q40.8 -12.9 42.4 -17.2Q44 -21.6 44 -27.2Q44 -32.9 42.4 -37.2Q40.8 -41.4 37.7 -43.8Q34.5 -46.1 29.9 -46.1Q25.2 -46.1 22 -43.6Q18.8 -41.2 17.3 -36.9Q15.7 -32.6 15.7 -27.2Q15.7 -21.8 17.3 -17.5Q18.9 -13.1 22.1 -10.6Q25.2 -8.1 29.9 -8.1Z",
    ),
    (
        38.7,
        "M7.2 0V-54.6H17.6V-45.8H18.2Q19.7 -50.3 23.5 -52.8Q27.2 -55.4 32 -55.4Q33.1 -55.4 34.4 -55.3Q35.7 -55.2 36.6 -55.1V-44.8Q36 -45 34.3 -45.2Q32.7 -45.4 30.9 -45.4Q27.2 -45.4 24.3 -43.8Q21.3 -42.3 19.6 -39.5Q18 -36.7 18 -33.1V0Z",
    ),
    (
        53.9,
        "M26.8 1.1Q20.9 1.1 16.3 -0.6Q11.7 -2.3 8.7 -5.6Q5.7 -9 4.8 -13.8L14.9 -15.9Q16 -11.6 19.1 -9.6Q22.2 -7.6 26.9 -7.6Q32.1 -7.6 35.2 -9.7Q38.3 -11.9 38.3 -15Q38.3 -17.6 36.4 -19.3Q34.5 -21 30.6 -21.8L21.4 -23.9Q13.8 -25.6 10.1 -29.4Q6.4 -33.1 6.4 -39Q6.4 -43.9 9.1 -47.5Q11.8 -51.2 16.6 -53.2Q21.3 -55.3 27.5 -55.3Q33.4 -55.3 37.6 -53.6Q41.8 -51.8 44.4 -48.8Q47 -45.7 48.2 -41.7L38.5 -39.5Q37.5 -42.4 35 -44.6Q32.4 -46.8 27.6 -46.8Q23.1 -46.8 20.2 -44.8Q17.2 -42.8 17.2 -39.7Q17.2 -37 19.2 -35.3Q21.2 -33.6 25.6 -32.6L34.4 -30.6Q42 -28.9 45.7 -25.2Q49.3 -21.6 49.3 -15.9Q49.3 -10.9 46.5 -7.1Q43.6 -3.2 38.5 -1Q33.5 1.1 26.8 1.1Z",
    ),
    (
        34.0,
        "M30.6 -54.6V-45.9H1V-54.6ZM9 -67.6H19.7V-14.8Q19.7 -11.5 21.1 -9.9Q22.5 -8.4 25.6 -8.4Q26.5 -8.4 27.8 -8.6Q29.2 -8.8 30.2 -9L32.1 -0.4Q30.4 0.2 28.4 0.5Q26.5 0.7 24.6 0.7Q17.1 0.7 13 -3.1Q9 -6.9 9 -13.9Z",
    ),
    (
        60.2,
        "M25.8 0.7Q20.3 0.7 16.1 -1.7Q11.9 -4 9.5 -8.6Q7.2 -13.2 7.2 -19.9V-54.6H18V-21.2Q18 -15.3 21.1 -12Q24.1 -8.7 29.5 -8.7Q33.1 -8.7 36 -10.3Q38.9 -11.8 40.5 -14.9Q42.2 -18 42.2 -22.2V-54.6H53V0H42.7L42.6 -13.4H43.7Q41.3 -6 36.7 -2.6Q32.1 0.7 25.8 0.7Z",
    ),
    (
        57.5,
        "M22.9 0 2.3 -54.6H13.9L24.7 -23.6Q26.5 -18.4 27.7 -13.3Q29 -8.2 30.4 -3.1H27.2Q28.6 -8.2 29.9 -13.3Q31.1 -18.4 32.9 -23.6L43.6 -54.6H55.2L34.6 0Z",
    ),
    (
        82.9,
        "M19.2 0 2.8 -54.6H14.1L20.1 -32Q21.4 -26.9 23 -20.7Q24.5 -14.4 25.9 -6.7H24.3Q25.7 -14.2 27.2 -20.4Q28.7 -26.7 30.1 -32L36.2 -54.6H46.9L52.8 -32Q54.1 -26.8 55.6 -20.6Q57.1 -14.3 58.5 -6.7H57Q58.4 -14.2 59.9 -20.5Q61.3 -26.7 62.7 -32L68.7 -54.6H80.1L63.7 0H52.9L46.1 -23.5Q45.1 -27 44.2 -30.9Q43.2 -34.8 42.2 -38.9Q41.3 -42.9 40.3 -46.9H42.5Q41.6 -43 40.6 -38.9Q39.7 -34.8 38.7 -30.9Q37.7 -27 36.7 -23.5L30 0Z",
    ),
    (
        55.7,
        "M2.8 0 24.2 -32.2 24.2 -23.9 3.9 -54.6H16L22.2 -44.7Q24.7 -40.5 26.6 -36.8Q28.6 -33 30.6 -29.5H25.6Q27.7 -33 29.5 -36.8Q31.3 -40.5 33.9 -44.7L40.2 -54.6H52.1L31.6 -23.6V-31.6L52.9 0H40.9L33.5 -11.6Q31 -15.7 29.1 -19.3Q27.2 -23 25.2 -26.4H30.2Q28.2 -23 26.5 -19.3Q24.7 -15.7 22.2 -11.6L14.8 0Z",
    ),
    (
        57.5,
        "M6.5 19.6 9.1 10.9 10.3 11.2Q13.1 11.9 15.3 11.6Q17.6 11.2 19.2 9.5Q20.8 7.8 21.8 4.3L23 0.2L2.3 -54.6H13.9L24.7 -23.6Q26.5 -18.4 27.7 -13.3Q28.9 -8.2 30.2 -3.1H27Q28.4 -8.2 29.7 -13.3Q31 -18.4 32.7 -23.6L43.8 -54.6H55.3L31.5 7.6Q29.9 11.9 27.5 14.9Q25.1 17.8 21.7 19.3Q18.4 20.8 14 20.8Q11.5 20.8 9.5 20.4Q7.5 20.1 6.5 19.6Z",
    ),
    (
        55.9,
        "M6 0V-7.3L35.4 -44.7V-45.2H7V-54.6H48.9V-46.7L20.6 -9.9V-9.4H49.9V0Z",
    ),
    (
        44.0,
        "M7 -31.4V-36.9Q13 -36.9 15.3 -39.4Q17.7 -41.9 17.7 -47.5V-57.9Q17.7 -63.3 18.9 -67Q20.2 -70.8 22.7 -73Q25.3 -75.2 29.2 -76.2Q33.1 -77.2 38.4 -77.2V-68.6Q32.2 -68.6 29.9 -65.9Q27.6 -63.2 27.6 -57.3V-44.4Q27.6 -41.7 26.8 -39.3Q26 -36.9 23.8 -35.2Q21.6 -33.4 17.6 -32.4Q13.5 -31.4 7 -31.4ZM38.4 13.6Q33.1 13.6 29.2 12.7Q25.3 11.7 22.7 9.5Q20.2 7.2 18.9 3.5Q17.7 -0.2 17.7 -5.7V-16Q17.7 -21.7 15.3 -24.2Q13 -26.7 7 -26.7V-32.2Q13.5 -32.2 17.6 -31.2Q21.6 -30.2 23.8 -28.4Q26 -26.6 26.8 -24.2Q27.6 -21.8 27.6 -19.1V-6.3Q27.6 -0.4 29.9 2.3Q32.2 5.1 38.4 5.1ZM7 -26.6V-37H17.1V-26.6Z",
    ),
    (34.6, "M22.1 -96.2V23.4H12.4V-96.2Z"),
    (
        44.0,
        "M37.1 -31.4Q30.5 -31.4 26.5 -32.4Q22.4 -33.4 20.2 -35.2Q18 -36.9 17.2 -39.3Q16.4 -41.7 16.4 -44.4V-57.3Q16.4 -63.1 14.2 -65.9Q11.9 -68.6 5.7 -68.6V-77.2Q10.9 -77.2 14.8 -76.2Q18.7 -75.2 21.3 -73Q23.8 -70.8 25.1 -67Q26.3 -63.3 26.3 -57.9V-47.5Q26.3 -41.9 28.7 -39.4Q31.1 -36.9 37.1 -36.9ZM5.7 13.6V5.1Q11.9 5.1 14.2 2.3Q16.4 -0.4 16.4 -6.3V-19.1Q16.4 -21.8 17.2 -24.2Q18 -26.6 20.2 -28.4Q22.4 -30.2 26.5 -31.2Q30.5 -32.2 37.1 -32.2V-26.7Q31 -26.6 28.7 -24.1Q26.3 -21.6 26.3 -16V-5.7Q26.3 -0.2 25.1 3.5Q23.8 7.2 21.3 9.5Q18.7 11.7 14.8 12.7Q10.9 13.6 5.7 13.6ZM37.1 -26.7 26.9 -26.6V-37L37.1 -36.9Z",
    ),
    (
        66.7,
        "M8 -20.9Q7.9 -27.2 9.9 -31.1Q11.8 -35.1 15.2 -37Q18.6 -38.8 22.6 -38.8Q26.1 -38.8 29.1 -37.4Q32.1 -35.9 36 -32.4Q38.5 -30.3 40 -29.4Q41.5 -28.4 43.3 -28.4Q46.2 -28.4 47.9 -30.6Q49.6 -32.7 49.6 -37H58.7Q58.8 -30.7 56.8 -26.8Q54.8 -22.8 51.5 -20.9Q48.1 -19.1 44.2 -19.1Q40.5 -19.1 37.5 -20.6Q34.4 -22.2 30.7 -25.4Q28.2 -27.6 26.7 -28.5Q25.2 -29.4 23.4 -29.4Q20.7 -29.4 18.9 -27.4Q17.1 -25.5 17.2 -20.9Z",
    ),
];

type VerticalTextCache = HashMap<u64, Arc<[u8]>>;
static VERTICAL_TEXT_CACHE: LazyLock<Mutex<VerticalTextCache>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn hash_vertical_key(label: &str, rgb_u24: u32, font_px_x10: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in label.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h ^= u64::from(rgb_u24) << 16;
    h = h.wrapping_mul(0x0100_0000_01b3);
    h ^= u64::from(font_px_x10);
    h.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

fn render_vertical_svg_text(label: &str, fg: Hsla, font_size: f32) -> AnyElement {
    use std::fmt::Write as _;
    let rgba: Rgba = fg.into();
    let r = (rgba.r.clamp(0., 1.) * 255.).round() as u32;
    let g = (rgba.g.clamp(0., 1.) * 255.).round() as u32;
    let b = (rgba.b.clamp(0., 1.) * 255.).round() as u32;
    let alpha = rgba.a.clamp(0., 1.);
    let rgb_u24 = (r << 16) | (g << 8) | b;
    let font_px_x10 = (font_size * 10.).round() as u32;
    let cache_key = hash_vertical_key(label, rgb_u24, font_px_x10);

    let bytes = {
        let mut guard = VERTICAL_TEXT_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = guard.get(&cache_key) {
            existing.clone()
        } else {
            let em_scale = (font_size / 100.0).clamp(0.08, 0.20);
            let max_pen = (162.0 / em_scale).max(400.0);
            let mut glyphs = String::with_capacity(label.len() * 192);
            let mut pen_x = 0.0f32;
            for ch in label.chars() {
                let idx = if (' '..='~').contains(&ch) {
                    (ch as usize) - 32
                } else if ch == '–' || ch == '—' {
                    '-' as usize - 32
                } else {
                    0
                };
                let (adv, d) = INTER_MEDIUM_ASCII[idx];
                if pen_x + adv > max_pen {
                    break;
                }
                if !d.is_empty() {
                    let _ = write!(
                        glyphs,
                        r#"<path transform="translate({pen_x:.1} 0)" d="{d}"/>"#
                    );
                }
                pen_x += adv + 1.2;
            }
            let doc = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="176" viewBox="0 0 24 176"><g transform="translate(16.2 170) rotate(-90) scale({em_scale:.4})" fill="#{r:02x}{g:02x}{b:02x}">{glyphs}</g></svg>"##
            );
            let arc: Arc<[u8]> = Arc::from(doc.into_bytes());
            if guard.len() >= 256 {
                guard.clear();
            }
            guard.insert(cache_key, arc.clone());
            arc
        }
    };

    let svg_el = pro_svg_doc(cache_key, bytes, 24., 176.);
    if alpha < 0.999 {
        div()
            .w(px(24.))
            .h(px(176.))
            .flex_none()
            .opacity(alpha)
            .child(svg_el)
            .into_any_element()
    } else {
        svg_el
    }
}

fn render_plus_glyph(side: f32, stroke_w: f32, color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let bw = f32::from(bounds.size.width);
            let bh = f32::from(bounds.size.height);
            if bw <= 2. || bh <= 2. {
                return;
            }
            let cx_pt = bounds.left() + px(bw * 0.5);
            let cy_pt = bounds.top() + px(bh * 0.5);
            let arm = (bw.min(bh) * 0.38).max(2.5);
            let mut pb = PathBuilder::stroke(px(stroke_w));
            pb.move_to(point(cx_pt - px(arm), cy_pt));
            pb.line_to(point(cx_pt + px(arm), cy_pt));
            pb.move_to(point(cx_pt, cy_pt - px(arm)));
            pb.line_to(point(cx_pt, cy_pt + px(arm)));
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(side))
    .flex_none()
}

fn render_filter_bars_glyph(side: f32, color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let bw = f32::from(bounds.size.width);
            let bh = f32::from(bounds.size.height);
            if bw <= 2. || bh <= 2. {
                return;
            }
            let mid_x = bounds.left() + px(bw * 0.5);
            let mut pb = PathBuilder::stroke(px(1.35));
            for (row_y, half_w) in [(0.27, 0.40), (0.50, 0.27), (0.73, 0.15)] {
                let y = bounds.top() + px(bh * row_y);
                let hw = px(bw * half_w);
                pb.move_to(point(mid_x - hw, y));
                pb.line_to(point(mid_x + hw, y));
            }
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(side))
    .flex_none()
}

fn parse_kpi_numeric(raw: &str) -> Option<(String, f64, usize, String)> {
    let trimmed = raw.trim();
    let num_start = trimmed.find(|c: char| c.is_ascii_digit() || c == '-')?;
    let prefix = trimmed[..num_start].to_owned();
    let rest = &trimmed[num_start..];
    let mut num_end = 0usize;
    let mut saw_dot = false;
    for (i, ch) in rest.char_indices() {
        if ch.is_ascii_digit() || (i == 0 && ch == '-') || ch == ',' {
            num_end = i + ch.len_utf8();
        } else if ch == '.' && !saw_dot {
            saw_dot = true;
            num_end = i + 1;
        } else {
            break;
        }
    }
    let num_slice = rest[..num_end].replace(',', "");
    let val: f64 = num_slice.parse().ok()?;
    let decimals = num_slice
        .split_once('.')
        .map(|(_, frac)| frac.len())
        .unwrap_or(0);
    let suffix = rest[num_end..].to_owned();
    Some((prefix, val, decimals, suffix))
}

#[derive(IntoElement)]
struct ContextNoteEntrance {
    id: ElementId,
    parent_gap: f32,
    content: AnyElement,
}

impl RenderOnce for ContextNoteEntrance {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let st = w.use_keyed_state(pro_eid(&self.id, "note-enter"), cx, |_, inner_cx| {
            let mut slide = Slide::new(0.);
            slide.duration(1., 0.26, inner_cx);
            (slide, Rc::new(Cell::new(px(36.))))
        });
        let (amt, h_cell) = st.update(cx, |s, cx| (s.0.get(w, cx).clamp(0., 1.), s.1.clone()));
        let h = h_cell.get();
        div()
            .id(self.id)
            .mt(px(-self.parent_gap * (1. - amt)))
            .w_full()
            .flex_none()
            .when(amt < 0.998, |v| v.h(h * amt).overflow_hidden())
            .opacity(amt)
            .child(
                div()
                    .relative()
                    .top(px(6. * (1. - amt)))
                    .w_full()
                    .flex_none()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first()
                            && b.size.height > px(0.)
                        {
                            h_cell.set(b.size.height);
                        }
                    })
                    .child(self.content),
            )
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
    user_notes: Vec<(usize, SharedString, SharedString)>,
    next_note_seq: usize,
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
            next_note_seq: 0,
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
        let seq = self.next_note_seq;
        self.next_note_seq += 1;
        self.user_notes
            .insert(0, (seq, item_id.clone(), text.clone()));
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
        let sat = saturation.clamp(0.18, 1.0);
        let base_top: Hsla = if dark {
            rgb(0x232327).into()
        } else {
            rgb(0xf4f4f7).into()
        };
        let base_bottom: Hsla = if dark {
            rgb(0x141417).into()
        } else {
            rgb(0xe7e7ed).into()
        };
        let neutral_mid = pro_mix_color(base_top, base_bottom, 0.45);
        let strong_tinted = pro_mix_color(neutral_mid, strong, sat);
        let soft_tinted = pro_mix_color(neutral_mid, soft, sat);
        let streak_tint = pro_mix_color(
            rgb(0xffffff).into(),
            soft,
            if dark { 0.26 * sat } else { 0.08 * sat },
        );

        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 1. || bh <= 1. {
                    return;
                }
                let r = px(12.);
                // 1. Base vertical brushed-metal surface gradient
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
                // 2. Top-left signature wash (fading to matching RGB at 0 alpha — never black mud)
                let strong_a = if dark { 0.56 * sat } else { 0.58 * sat };
                window.paint_quad(
                    fill(
                        bounds,
                        linear_gradient(
                            135.,
                            linear_color_stop(strong_tinted.opacity(strong_a), 0.),
                            linear_color_stop(strong_tinted.opacity(0.), 0.64),
                        ),
                    )
                    .corner_radii(r),
                );
                // 3. Bottom-right soft atmospheric wash
                let soft_a = if dark { 0.24 * sat } else { 0.56 * sat };
                window.paint_quad(
                    fill(
                        bounds,
                        linear_gradient(
                            318.,
                            linear_color_stop(soft_tinted.opacity(soft_a), 0.),
                            linear_color_stop(soft_tinted.opacity(0.), 0.68),
                        ),
                    )
                    .corner_radii(r),
                );
                // 4. Concentric multi-stop Gaussian diagonal brushed-metallic specular streak
                let xl = bounds.left() - px(6.);
                let xr = bounds.right() + px(6.);
                let yl = bounds.top() + px(bh * 0.80);
                let yr = bounds.top() + px(bh * 0.28);
                let bands: [(f32, f32, f32); 5] = [
                    (0.24, 0.026, 0.14),
                    (0.15, 0.032, 0.16),
                    (0.09, 0.038, 0.18),
                    (0.048, 0.044, 0.20),
                    (0.020, 0.038, 0.18),
                ];
                for (half_h_frac, a_dark, a_light) in bands {
                    let dy = px(bh * half_h_frac);
                    let a = if dark { a_dark } else { a_light };
                    let mut pb = PathBuilder::fill();
                    pb.move_to(point(xl, yl - dy));
                    pb.line_to(point(xr, yr - dy));
                    pb.line_to(point(xr, yr + dy));
                    pb.line_to(point(xl, yr + dy + (yl - yr)));
                    pb.close();
                    if let Ok(path) = pb.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                126.,
                                linear_color_stop(streak_tint.opacity(a * 0.55), 0.),
                                linear_color_stop(streak_tint.opacity(a), 0.78),
                            ),
                        );
                    }
                }
                // 5. Subtle top specular rim sheen
                let rim_col = if dark {
                    hsla(0., 0., 1., 0.07)
                } else {
                    hsla(0., 0., 1., 0.65)
                };
                window.paint_quad(
                    fill(
                        Bounds::new(bounds.origin, size(bounds.size.width, px(28.))),
                        linear_gradient(
                            180.,
                            linear_color_stop(rim_col, 0.),
                            linear_color_stop(rim_col.opacity(0.), 1.),
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
                let pad_x = 2.0;
                let pad_y = 2.2;
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
                    point(last_pt.x - px(1.7), last_pt.y - px(1.7)),
                    size(px(3.4), px(3.4)),
                );
                window.paint_quad(fill(dot_bounds, dot).corner_radii(px(99.)));
            },
        )
        .w(px(34.))
        .h(px(14.))
        .flex_none()
    }

    fn render_slim_label(
        label: &str,
        hovered: bool,
        far: bool,
        compact: bool,
        p: &ProPalette,
        _cx: &App,
    ) -> AnyElement {
        let fg = if hovered {
            p.ink
        } else if far {
            p.muted
        } else {
            p.text
        };
        let is_cjk = label.chars().any(|c| c > '\u{2e7f}');
        if is_cjk {
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
            render_vertical_svg_text(label, fg, if compact { 11.0 } else { 12.0 })
        }
    }

    fn render_product_screen(
        &self,
        item: &GalleryItem,
        open_amt: f32,
        is_active: bool,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let screen_card: Hsla = if p.dark {
            rgb(0x1b1b1e).into()
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
            .size(px(18.))
            .rounded(px(5.))
            .flex_none()
            .bg(p.primary);

        let id_sfx = if is_active {
            String::new()
        } else {
            format!("-cls-{}", item.id)
        };

        let switcher = pro_tactile(
            pro_eid(&self.id, format!("screen-switcher{id_sfx}")),
            tr(cx, "Switch showcase agent", "切换展示智能体"),
            6.,
            screen_card,
            p.hover,
            p.line_strong,
            p.line_strong,
            false,
            div()
                .h(px(26.))
                .pl(px(6.))
                .pr(px(7.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(small_mark)
                .child(pro_swap_label(
                    pro_eid(&self.id, format!("screen-sw-lbl{id_sfx}")),
                    display_name.clone(),
                    12.,
                    FontWeight::MEDIUM,
                    p.ink,
                ))
                .child(pro_svg("0 0 24 24", CHEVRON_UP_DOWN_SVG, 11., p.muted)),
            cx,
            |this, _, cx| {
                this.select_next(cx);
            },
        )
        .flex_none();

        let new_agent_lbl = tr(cx, "New agent", "新建智能体");
        let new_agent_btn = pro_tactile(
            pro_eid(&self.id, format!("screen-new-agent{id_sfx}")),
            new_agent_lbl,
            7.,
            p.primary,
            pro_mix_color(p.primary, p.on_primary, 0.12),
            transparent_black(),
            transparent_black(),
            false,
            div()
                .h(px(28.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.on_primary)
                .child(render_plus_glyph(11., 1.65, p.on_primary))
                .child(new_agent_lbl),
            cx,
            |this, _, cx| {
                this.toast = Some((
                    tr(cx, "Opened new agent template", "已打开新建智能体模板").into(),
                    Instant::now(),
                ));
                cx.emit(AgentGalleryEvent::NewAgentRequested);
                cx.notify();
            },
        )
        .flex_none();

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
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("screen-sync-lbl{id_sfx}")),
                        item.synced_ago.clone(),
                        11.5,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );

        let kpi_count = if compact { 2 } else { 4 };
        let mut kpi_row = div().mt(px(16.)).flex().items_start().gap(px(12.));
        for (kpi_idx, kpi) in item.kpis.iter().take(kpi_count).enumerate() {
            let val_el =
                if is_active && let Some((pfx, num, dec, sfx)) = parse_kpi_numeric(&kpi.value) {
                    pro_rolling_number(
                        pro_eid(&self.id, format!("screen-kpi-roll-{kpi_idx}")),
                        num,
                        dec,
                        pfx,
                        sfx,
                        13.,
                        FontWeight::SEMIBOLD,
                        p.strong,
                    )
                } else {
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.strong)
                        .child(kpi.value.clone())
                        .into_any_element()
                };

            kpi_row = kpi_row.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .w_full()
                            .text_size(px(9.5))
                            .line_height(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .whitespace_nowrap()
                            .child(translate_kpi_label(&kpi.label, cx).to_owned()),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.))
                            .child(val_el)
                            .child(Self::render_mini_sparkline(&kpi.spark, p.faint, p.muted)),
                    ),
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
                pro_eid(&self.id, format!("screen-tabs-pill{id_sfx}")),
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
            let tab_lbl = tab.label(cx);
            tabs_bar = tabs_bar.child(
                pro_tactile(
                    pro_eid(&self.id, format!("screen-tab-{}{id_sfx}", tab.key())),
                    tab_lbl,
                    6.,
                    transparent_black(),
                    if active { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    active,
                    div()
                        .h(px(25.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if active { p.ink } else { p.text })
                        .child(tab_lbl),
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
                )
                .relative()
                .flex_none()
                .when(active, |d| d.child(pro_mark_selected(&tab_sensor))),
            );
        }

        let tab_body = match self.screen_tab {
            GalleryScreenTab::Context => {
                self.render_context_tab(item, &id_sfx, screen_card, &p, cx)
            }
            GalleryScreenTab::Runs => self.render_runs_tab(item, &id_sfx, screen_card, &p, cx),
            GalleryScreenTab::Tools => self.render_tools_tab(item, screen_card, &p, cx),
            GalleryScreenTab::Settings => self.render_settings_tab(item, screen_card, &p, cx),
        };

        let page = div()
            .px(px(if compact { 18. } else { 22. }))
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
                pro_eid(&self.id, format!("screen-tab-vtx{id_sfx}")),
                format!("{}-{}", item.id, self.screen_tab.key()),
                tab_body,
            ));

        let inset_left = if compact { 18. } else { 30. };
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
            .shadow(pro_card_shadow(p.dark))
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
        id_sfx: &str,
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
            .child(
                pro_tactile(
                    pro_eid(&self.id, format!("screen-ctx-filter{id_sfx}")),
                    tr(cx, "Toggle context filter", "切换上下文筛选"),
                    6.,
                    screen_card,
                    p.hover,
                    p.line,
                    p.line_strong,
                    !self.search_filter_all,
                    div()
                        .h(px(27.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .whitespace_nowrap()
                        .child(render_filter_bars_glyph(12., p.muted))
                        .child(pro_swap_label(
                            pro_eid(&self.id, format!("screen-ctx-flbl{id_sfx}")),
                            filter_label,
                            12.,
                            FontWeight::MEDIUM,
                            p.ink,
                        )),
                    cx,
                    |this, _, cx| {
                        this.search_filter_all = !this.search_filter_all;
                        cx.notify();
                    },
                )
                .flex_none(),
            );

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
                    .child(pro_btn(
                        pro_eid(&self.id, format!("screen-add-ctx{id_sfx}")),
                        tr(cx, "Add context", "添加上下文"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
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

        for (seq, note_item_id, note_text) in &self.user_notes {
            if note_item_id == &item.id {
                let row = div()
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
                    );
                feed = feed.child(ContextNoteEntrance {
                    id: pro_eid(&self.id, format!("unote-{seq}{id_sfx}")),
                    parent_gap: 10.,
                    content: row.into_any_element(),
                });
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
        id_sfx: &str,
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
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("step-chk-{}-{idx}{id_sfx}", item.id)),
                        true,
                        16.,
                        false,
                        p.green,
                    ))
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
            50.
        } else {
            62.
        };
        let gap = if narrow { 8. } else { 10. };
        let open_w = if narrow {
            (container_w - 2. * (side_w + gap) - 20.).max(256.)
        } else {
            (container_w * 0.555).clamp(492., 620.)
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
                    .shadow(pro_card_shadow(p.dark))
                    .overflow_hidden()
                    .cursor_pointer()
                    .child(Self::render_panel_artwork(
                        item.colors[0],
                        item.colors[1],
                        0.32,
                        p.dark,
                    ))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .pb(px(12.))
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
                0.58
            } else if dist == 2 {
                0.40
            } else {
                0.32
            };
            let sat = (base_sat + (1.0 - base_sat) * open_amt.max(hov_amt * 0.85)).clamp(0.24, 1.0);

            let item_snapshot = self.items[idx].clone();
            let label_str: SharedString = if self.custom_items {
                item_snapshot.name.clone()
            } else {
                translate_agent_name(&item_snapshot.id, &item_snapshot.name, cx).into()
            };

            let is_open = idx == self.active_index;
            let screen_el = if open_amt > 0.02 {
                Some(self.render_product_screen(
                    &item_snapshot,
                    open_amt,
                    is_open,
                    ultra_narrow,
                    cx,
                ))
            } else {
                None
            };

            let label_alpha = (1.0 - open_amt * 2.5).clamp(0., 1.);

            let mut panel = div()
                .id(pro_eid(&self.id, format!("panel-{idx}")))
                .w(px(cur_w))
                .h_full()
                .flex_none()
                .relative()
                .rounded(px(12.))
                .border_1()
                .border_color(if is_open { p.line_strong } else { p.line })
                .shadow(pro_card_shadow(p.dark))
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
                        .pb(px(if narrow { 10. } else { 12. }))
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
                    .shadow(pro_card_shadow(p.dark))
                    .overflow_hidden()
                    .cursor_pointer()
                    .child(Self::render_panel_artwork(
                        item.colors[0],
                        item.colors[1],
                        0.32,
                        p.dark,
                    ))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .pb(px(12.))
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

        let prev_btn = pro_icon_btn(
            pro_eid(&self.id, "prev-agent"),
            tr(cx, "Previous agent", "上一个智能体"),
            ProIcon::ChevronLeft,
            28.,
            false,
            cx,
            |this, _, cx| {
                this.select_prev(cx);
            },
        )
        .bg(p.card)
        .border_1()
        .border_color(p.line_strong);

        let next_btn = pro_icon_btn(
            pro_eid(&self.id, "next-agent"),
            tr(cx, "Next agent", "下一个智能体"),
            ProIcon::ChevronRight,
            28.,
            false,
            cx,
            |this, _, cx| {
                this.select_next(cx);
            },
        )
        .bg(p.card)
        .border_1()
        .border_color(p.line_strong);

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
