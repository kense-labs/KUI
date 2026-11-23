//! Interactive preview stories for KUI Pro motion and full-application screens.
use gpui::{prelude::*, *};
use kui::{components::gradient_floor::GradientFloor, foundation::Theme};

pub const PREVIEW_HEIGHT: f32 = 896.;
pub const FLOOR_PREVIEW_HEIGHT: f32 = 408.;

pub const IDS: &[&str] = &[
    "gradient-floor",
    "app-shell",
    "app-shell-flat",
    "agent-activity",
    "agents",
    "inbox",
    "agent-inbox-board",
    "agent-memory",
    "agent-orchestration",
    "agent-permissions",
    "agent-run-detail",
    "agent-schedules",
    "agent-skills",
    "agent-tasks",
    "agent-thread",
    "agent-tool-calls",
    "agent-usage",
    "assistant-home",
    "assistant-overview",
    "agent-chat",
    "agent-chat-rail",
    "agent-chat-progress",
    "agent-run",
    "ai-chat-revenue",
    "ai-chat-spend",
    "onboarding",
    "onboarding-website",
    "onboarding-brand",
    "onboarding-tone",
    "onboarding-ready",
    "settings-general",
    "settings-profile",
    "settings-preferences",
    "settings-models",
    "settings-members",
    "settings-subscription",
    "settings-notifications",
    "settings-privacy",
    "settings-security",
    "analytics",
    "crm",
    "evals",
    "files",
    "folder",
    "integrations",
    "kanban",
    "workflow-builder",
];

pub const SCREEN_IDS: &[&str] = &[
    "app-shell",
    "app-shell-flat",
    "agent-activity",
    "agents",
    "inbox",
    "agent-inbox-board",
    "agent-memory",
    "agent-orchestration",
    "agent-permissions",
    "agent-run-detail",
    "agent-schedules",
    "agent-skills",
    "agent-tasks",
    "agent-thread",
    "agent-tool-calls",
    "agent-usage",
    "assistant-home",
    "assistant-overview",
    "agent-chat",
    "agent-chat-rail",
    "agent-chat-progress",
    "agent-run",
    "ai-chat-revenue",
    "ai-chat-spend",
    "onboarding",
    "onboarding-website",
    "onboarding-brand",
    "onboarding-tone",
    "onboarding-ready",
    "settings-general",
    "settings-profile",
    "settings-preferences",
    "settings-models",
    "settings-members",
    "settings-subscription",
    "settings-notifications",
    "settings-privacy",
    "settings-security",
    "analytics",
    "crm",
    "evals",
    "files",
    "folder",
    "integrations",
    "kanban",
    "workflow-builder",
];

struct GradientFloorStory {
    floor: Entity<GradientFloor>,
}

impl Render for GradientFloorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = Theme::global(cx).mode == kui::foundation::ThemeMode::Dark;
        let border = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.07)
        };
        div()
            .size_full()
            .px(px(24.))
            .py(px(24.))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .h(px(360.))
                    .rounded(px(20.))
                    .overflow_hidden()
                    .border_1()
                    .border_color(border)
                    .child(self.floor.clone()),
            )
    }
}

pub fn story(
    id: &str,
    _window: &mut Window,
    cx: &mut Context<crate::StoryHost>,
) -> Option<AnyView> {
    match id {
        "gradient-floor" => Some(
            cx.new(|cx| {
                cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
                let floor = cx.new(|cx| {
                    cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
                    GradientFloor::new("gradient-floor-preview")
                        .with_height(px(360.))
                        .with_radius(px(20.))
                });
                GradientFloorStory { floor }
            })
            .into(),
        ),
        "app-shell" => Some(
            cx.new(|cx| kui::components::app_shell::AppShell::new("app-shell", cx))
                .into(),
        ),
        "app-shell-flat" => Some(
            cx.new(|cx| kui::components::app_shell_flat::AppShellFlat::new("app-shell-flat", cx))
                .into(),
        ),
        "agent-activity" => Some(
            cx.new(|cx| {
                kui::components::agent_activity::AgentActivityScreen::new("agent-activity", cx)
            })
            .into(),
        ),
        "agents" => Some(
            cx.new(|cx| kui::components::agents::AgentsScreen::new("agents", cx))
                .into(),
        ),
        "inbox" => Some(
            cx.new(|cx| kui::components::inbox::AgentInboxScreen::new("inbox", cx))
                .into(),
        ),
        "agent-inbox-board" => Some(
            cx.new(|cx| {
                kui::components::agent_inbox_board::AgentInboxBoardScreen::new(
                    "agent-inbox-board",
                    cx,
                )
            })
            .into(),
        ),
        "agent-memory" => Some(
            cx.new(|cx| kui::components::agent_memory::AgentMemoryScreen::new("agent-memory", cx))
                .into(),
        ),
        "agent-orchestration" => Some(
            cx.new(|cx| {
                kui::components::agent_orchestration::AgentOrchestrationScreen::new(
                    "agent-orchestration",
                    cx,
                )
            })
            .into(),
        ),
        "agent-permissions" => Some(
            cx.new(|cx| {
                kui::components::agent_permissions::AgentPermissionsScreen::new(
                    "agent-permissions",
                    cx,
                )
            })
            .into(),
        ),
        "agent-run-detail" => Some(
            cx.new(|cx| {
                kui::components::agent_run_detail::AgentRunDetailScreen::new("agent-run-detail", cx)
            })
            .into(),
        ),
        "agent-schedules" => Some(
            cx.new(|cx| {
                kui::components::agent_schedules::AgentSchedulesScreen::new("agent-schedules", cx)
            })
            .into(),
        ),
        "agent-skills" => Some(
            cx.new(|cx| kui::components::agent_skills::AgentSkillsScreen::new("agent-skills", cx))
                .into(),
        ),
        "agent-tasks" => Some(
            cx.new(|cx| kui::components::agent_tasks::AgentTasksScreen::new("agent-tasks", cx))
                .into(),
        ),
        "agent-thread" => Some(
            cx.new(|cx| kui::components::agent_thread::AgentThreadScreen::new("agent-thread", cx))
                .into(),
        ),
        "agent-tool-calls" => Some(
            cx.new(|cx| {
                kui::components::agent_tool_calls::AgentToolCallsScreen::new("agent-tool-calls", cx)
            })
            .into(),
        ),
        "agent-usage" => Some(
            cx.new(|cx| kui::components::agent_usage::AgentUsageScreen::new("agent-usage", cx))
                .into(),
        ),
        "assistant-home" => Some(
            cx.new(|cx| {
                kui::components::assistant_home::AssistantHomeScreen::new("assistant-home", cx)
            })
            .into(),
        ),
        "assistant-overview" => Some(
            cx.new(|cx| {
                kui::components::assistant_overview::AssistantOverviewScreen::new(
                    "assistant-overview",
                    cx,
                )
            })
            .into(),
        ),
        "agent-chat" => Some(
            cx.new(|cx| kui::components::agent_chat::AgentChatScreen::new("agent-chat", cx))
                .into(),
        ),
        "agent-chat-rail" => Some(
            cx.new(|cx| {
                kui::components::agent_chat_rail::AgentChatRailScreen::new("agent-chat-rail", cx)
            })
            .into(),
        ),
        "agent-chat-progress" => Some(
            cx.new(|cx| {
                kui::components::agent_chat_progress::AgentChatProgressScreen::new(
                    "agent-chat-progress",
                    cx,
                )
            })
            .into(),
        ),
        "agent-run" => Some(
            cx.new(|cx| kui::components::agent_run::AgentRunScreen::new("agent-run", cx))
                .into(),
        ),
        "ai-chat-revenue" => Some(
            cx.new(|cx| {
                kui::components::ai_chat_revenue::AiChatRevenueScreen::new("ai-chat-revenue", cx)
            })
            .into(),
        ),
        "ai-chat-spend" => Some(
            cx.new(|cx| {
                kui::components::ai_chat_spend::AiChatSpendScreen::new("ai-chat-spend", cx)
            })
            .into(),
        ),
        "onboarding" => Some(
            cx.new(|cx| kui::components::onboarding::OnboardingScreen::new("onboarding", cx))
                .into(),
        ),
        "onboarding-website" => Some(
            cx.new(|cx| {
                kui::components::onboarding_website::OnboardingWebsiteScreen::new(
                    "onboarding-website",
                    cx,
                )
            })
            .into(),
        ),
        "onboarding-brand" => Some(
            cx.new(|cx| {
                kui::components::onboarding_brand::OnboardingBrandScreen::new(
                    "onboarding-brand",
                    cx,
                )
            })
            .into(),
        ),
        "onboarding-tone" => Some(
            cx.new(|cx| {
                kui::components::onboarding_tone::OnboardingToneScreen::new("onboarding-tone", cx)
            })
            .into(),
        ),
        "onboarding-ready" => Some(
            cx.new(|cx| {
                kui::components::onboarding_ready::OnboardingReadyScreen::new(
                    "onboarding-ready",
                    cx,
                )
            })
            .into(),
        ),
        "settings-general" => Some(
            cx.new(|cx| {
                kui::components::settings_general::SettingsGeneralScreen::new(
                    "settings-general",
                    cx,
                )
            })
            .into(),
        ),
        "settings-profile" => Some(
            cx.new(|cx| {
                kui::components::settings_profile::SettingsProfileScreen::new(
                    "settings-profile",
                    cx,
                )
            })
            .into(),
        ),
        "settings-preferences" => Some(
            cx.new(|cx| {
                kui::components::settings_preferences::SettingsPreferencesScreen::new(
                    "settings-preferences",
                    cx,
                )
            })
            .into(),
        ),
        "settings-models" => Some(
            cx.new(|cx| {
                kui::components::settings_models::SettingsModelsScreen::new("settings-models", cx)
            })
            .into(),
        ),
        "settings-members" => Some(
            cx.new(|cx| {
                kui::components::settings_members::SettingsMembersScreen::new(
                    "settings-members",
                    cx,
                )
            })
            .into(),
        ),
        "settings-subscription" => Some(
            cx.new(|cx| {
                kui::components::settings_subscription::SettingsSubscriptionScreen::new(
                    "settings-subscription",
                    cx,
                )
            })
            .into(),
        ),
        "settings-notifications" => Some(
            cx.new(|cx| {
                kui::components::settings_notifications::SettingsNotificationsScreen::new(
                    "settings-notifications",
                    cx,
                )
            })
            .into(),
        ),
        "settings-privacy" => Some(
            cx.new(|cx| {
                kui::components::settings_privacy::SettingsPrivacyScreen::new(
                    "settings-privacy",
                    cx,
                )
            })
            .into(),
        ),
        "settings-security" => Some(
            cx.new(|cx| {
                kui::components::settings_security::SettingsSecurityScreen::new(
                    "settings-security",
                    cx,
                )
            })
            .into(),
        ),
        "analytics" => Some(
            cx.new(|cx| kui::components::analytics::AnalyticsScreen::new("analytics", cx))
                .into(),
        ),
        "crm" => Some(
            cx.new(|cx| kui::components::crm::CrmScreen::new("crm", cx))
                .into(),
        ),
        "evals" => Some(
            cx.new(|cx| kui::components::evals::EvalsScreen::new("evals", cx))
                .into(),
        ),
        "files" => Some(
            cx.new(|cx| kui::components::files::FilesScreen::new("files", cx))
                .into(),
        ),
        "folder" => Some(
            cx.new(|cx| kui::components::folder::FolderScreen::new("folder", cx))
                .into(),
        ),
        "integrations" => Some(
            cx.new(|cx| kui::components::integrations::IntegrationsScreen::new("integrations", cx))
                .into(),
        ),
        "kanban" => Some(
            cx.new(|cx| kui::components::kanban::KanbanScreen::new("kanban", cx))
                .into(),
        ),
        "workflow-builder" => Some(
            cx.new(|cx| {
                kui::components::workflow_builder::WorkflowBuilderScreen::new(
                    "workflow-builder",
                    cx,
                )
            })
            .into(),
        ),
        _ => None,
    }
}
