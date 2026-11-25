//! Interactive preview stories for KUI Pro motion, illustrations, blocks, and agent components.

use gpui::{prelude::*, *};
use kui::foundation::Theme;

pub const IDS: &[&str] = &[
    "blur-crossfade",
    "gradient-text",
    "tilt-card",
    "glass-folder",
    "glow-keyboard",
    "inspect-agent-card",
    "inspect-composer",
    "integration-flow",
    "model-menu",
    "outreach-branches",
    "progress-pill",
    "project-flow",
    "voice-capture",
    "world-map",
    "agent-gallery",
    "app-directory",
    "client-context",
    "invoice-credit",
    "kpi-board",
    "live-session",
    "metrics-dashboard",
    "mission-control",
    "mission-timeline",
    "pricing-plans",
    "revenue-chat",
    "run-inspector",
    "task-board",
    "ai-answer",
    "chat-composer",
    "cited-answer",
    "model-picker",
    "thinking-state",
    "writing-assistant",
    "command-menu",
    "integration-picker",
    "billing-schedule",
    "integration-settings",
    "prompt-library",
    "stepper-form",
    "dropzone",
    "breakdown-chart",
    "compact-breakdown",
    "experiment-card",
    "kpi-chart",
    "metering-chart",
    "model-usage",
    "trend-chart",
    "data-table",
    "project-card",
    "stat-card",
    "setup-guide",
    "toaster",
    "comments-panel",
    "agent-checkpoints",
    "agent-crew",
    "agent-cursors",
    "agent-findings",
    "agent-fleet",
    "agent-graph",
    "agent-inbox",
    "agent-lanes",
    "agent-ledger",
    "agent-metro-map",
    "agent-monitor",
    "agent-overlap-map",
    "agent-queue",
    "agent-recap",
    "agent-router",
    "agent-sequence",
    "agent-state-machine",
    "agent-steering",
    "agent-timing",
    "agent-trail",
    "agent-tree",
    "agent-workbench",
    "approval-card",
    "approval-gates",
    "autonomy-dial",
    "away-summary",
    "collaboration-stream",
    "commit-trail",
    "conflict-radar",
    "context-compaction",
    "context-map",
    "cost-waterfall",
    "decision-card",
    "decision-tree",
    "delegate-board",
    "diff-timeline",
    "fallback-chain",
    "fan-out-grid",
    "flow-builder",
    "handoff-swimlanes",
    "inline-diff",
    "judge-bracket",
    "memory-graph",
    "memory-panel",
    "model-router",
    "parallel-attempts",
    "permission-request",
    "plan-vs-actual",
    "rate-limit-planner",
    "reasoning-summary",
    "repo-map",
    "response-stream",
    "review-center",
    "root-cause-tree",
    "run-controls",
    "running-now",
    "scheduled-runs",
    "tool-access-map",
    "trace-waterfall",
    "voice-session",
];

struct AffectProStage {
    view: AnyView,
    full_block: bool,
}

impl AffectProStage {
    fn new(view: AnyView, full_block: bool, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        Self { view, full_block }
    }
}

impl Render for AffectProStage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = Theme::global(cx).mode == kui::foundation::ThemeMode::Dark;
        let stage_bg = if dark { rgb(0x1f1f22) } else { rgb(0xf4f4f6) };
        let vw = f32::from(window.viewport_size().width);
        let narrow = vw > 0. && vw < 520.;
        if self.full_block {
            div()
                .w_full()
                .flex_1()
                .bg(stage_bg)
                .flex()
                .flex_col()
                .items_stretch()
                .justify_center()
                .child(self.view.clone())
        } else {
            let px_pad = if narrow { 16. } else { 44. };
            let py_pad = if narrow { 20. } else { 44. };
            div()
                .w_full()
                .flex_1()
                .bg(stage_bg)
                .px(px(px_pad))
                .py(px(py_pad))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(self.view.clone())
        }
    }
}

pub fn story(
    id: &str,
    _window: &mut Window,
    cx: &mut Context<crate::StoryHost>,
) -> Option<AnyView> {
    match id {
        "blur-crossfade" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::blur_crossfade::BlurCrossfadeShowcase::new(
                            "blur-crossfade",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "gradient-text" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::gradient_text::GradientTextShowcase::new(
                            "gradient-text",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "tilt-card" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::tilt_card::TiltCardShowcase::new("tilt-card", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "glass-folder" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::glass_folder::GlassFolder::new("glass-folder", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "glow-keyboard" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::glow_keyboard::GlowKeyboard::new("glow-keyboard", cx)
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "inspect-agent-card" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::inspect_agent_card::InspectAgentCard::new(
                            "inspect-agent-card",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "inspect-composer" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::inspect_composer::InspectComposer::new(
                            "inspect-composer",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "integration-flow" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::integration_flow::IntegrationFlow::new(
                            "integration-flow",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "model-menu" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::model_menu::ModelMenu::new("model-menu", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "outreach-branches" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::outreach_branches::OutreachBranches::new(
                            "outreach-branches",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "progress-pill" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::progress_pill::ProgressPill::new("progress-pill", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "project-flow" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::project_flow::ProjectFlow::new("project-flow", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "voice-capture" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::voice_capture::VoiceCapture::new("voice-capture", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "world-map" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::world_map::WorldMap::new("world-map", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-gallery" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_gallery::AgentGalleryBlock::new("agent-gallery", cx)
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "app-directory" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::app_directory::AppDirectoryBlock::new("app-directory", cx)
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "client-context" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::client_context::ClientContextBlock::new(
                            "client-context",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "invoice-credit" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::invoice_credit::InvoiceCreditBlock::new(
                            "invoice-credit",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "kpi-board" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::kpi_board::KpiBoardBlock::new("kpi-board", cx))
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "live-session" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::live_session::LiveSessionBlock::new("live-session", cx)
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "metrics-dashboard" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::metrics_dashboard::MetricsDashboardBlock::new(
                            "metrics-dashboard",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "mission-control" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::mission_control::MissionControlBlock::new(
                            "mission-control",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "mission-timeline" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::mission_timeline::MissionTimelineBlock::new(
                            "mission-timeline",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "pricing-plans" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::pricing_plans::PricingPlansBlock::new("pricing-plans", cx)
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "revenue-chat" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::revenue_chat::RevenueChatBlock::new("revenue-chat", cx)
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "run-inspector" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::run_inspector::RunInspectorBlock::new("run-inspector", cx)
                    })
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "task-board" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::task_board::TaskBoardBlock::new("task-board", cx))
                    .into();
                AffectProStage::new(view, true, cx)
            })
            .into(),
        ),
        "ai-answer" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::ai_answer::AiAnswer::new("ai-answer", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "chat-composer" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::chat_composer::ChatComposer::new("chat-composer", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "cited-answer" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::cited_answer::CitedAnswer::new("cited-answer", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "model-picker" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::model_picker::ModelPicker::new("model-picker", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "thinking-state" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::thinking_state::ThinkingState::new("thinking-state", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "writing-assistant" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::writing_assistant::WritingAssistant::new(
                            "writing-assistant",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "command-menu" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::command_menu::CommandMenu::new("command-menu", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "integration-picker" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::integration_picker::IntegrationPicker::new(
                            "integration-picker",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "billing-schedule" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::billing_schedule::BillingSchedule::new(
                            "billing-schedule",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "integration-settings" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::integration_settings::IntegrationSettings::new(
                            "integration-settings",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "prompt-library" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::prompt_library::PromptLibrary::new("prompt-library", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "stepper-form" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::stepper_form::StepperForm::new("stepper-form", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "dropzone" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::dropzone::Dropzone::new("dropzone", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "breakdown-chart" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::breakdown_chart::BreakdownChart::new("breakdown-chart", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "compact-breakdown" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::compact_breakdown::CompactBreakdown::new(
                            "compact-breakdown",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "experiment-card" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::experiment_card::ExperimentCard::new("experiment-card", cx)
                            .with_default_open(true)
                            .with_default_metric("revenue")
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "kpi-chart" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::kpi_chart::KpiChart::new("kpi-chart", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "metering-chart" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::metering_chart::MeteringChart::new("metering-chart", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "model-usage" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::model_usage::ModelUsage::new("model-usage", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "trend-chart" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::trend_chart::TrendChart::new("trend-chart", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "data-table" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::data_table::DataTable::new("data-table", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "project-card" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::project_card::ProjectCard::new("project-card", cx)
                            .with_default_open(true)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "stat-card" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::stat_card::StatCard::new("stat-card", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "setup-guide" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::setup_guide::SetupGuide::new("setup-guide", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "toaster" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::toaster::Toaster::new("toaster", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "comments-panel" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::comments_panel::CommentsPanel::new("comments-panel", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-checkpoints" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_checkpoints::AgentCheckpoints::new(
                            "agent-checkpoints",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-crew" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_crew::AgentCrew::new("agent-crew", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-cursors" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_cursors::AgentCursors::new("agent-cursors", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-findings" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_findings::AgentFindings::new("agent-findings", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-fleet" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_fleet::AgentFleet::new("agent-fleet", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-graph" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_graph::AgentGraph::new("agent-graph", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-inbox" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_inbox::AgentInbox::new("agent-inbox", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-lanes" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_lanes::AgentLanes::new("agent-lanes", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-ledger" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_ledger::AgentLedger::new("agent-ledger", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-metro-map" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_metro_map::AgentMetroMap::new("agent-metro-map", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-monitor" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_monitor::AgentMonitor::new("agent-monitor", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-overlap-map" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_overlap_map::AgentOverlapMap::new(
                            "agent-overlap-map",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-queue" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_queue::AgentQueue::new("agent-queue", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-recap" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_recap::AgentRecap::new("agent-recap", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-router" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_router::AgentRouter::new("agent-router", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-sequence" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_sequence::AgentSequence::new("agent-sequence", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-state-machine" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_state_machine::AgentStateMachine::new(
                            "agent-state-machine",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-steering" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_steering::AgentSteering::new("agent-steering", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-timing" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_timing::AgentTiming::new("agent-timing", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-trail" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_trail::AgentTrail::new("agent-trail", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-tree" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::agent_tree::AgentTree::new("agent-tree", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "agent-workbench" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::agent_workbench::AgentWorkbench::new("agent-workbench", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "approval-card" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::approval_card::ApprovalCard::new("approval-card", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "approval-gates" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::approval_gates::ApprovalGates::new("approval-gates", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "autonomy-dial" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::autonomy_dial::AutonomyDial::new("autonomy-dial", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "away-summary" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::away_summary::AwaySummary::new("away-summary", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "collaboration-stream" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::collaboration_stream::CollaborationStream::new(
                            "collaboration-stream",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "commit-trail" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::commit_trail::CommitTrail::new("commit-trail", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "conflict-radar" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::conflict_radar::ConflictRadar::new("conflict-radar", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "context-compaction" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::context_compaction::ContextCompaction::new(
                            "context-compaction",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "context-map" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::context_map::ContextMap::new("context-map", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "cost-waterfall" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::cost_waterfall::CostWaterfall::new("cost-waterfall", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "decision-card" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::decision_card::DecisionCard::new("decision-card", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "decision-tree" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::decision_tree::DecisionTree::new("decision-tree", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "delegate-board" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::delegate_board::DelegateBoard::new("delegate-board", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "diff-timeline" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::diff_timeline::DiffTimeline::new("diff-timeline", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "fallback-chain" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::fallback_chain::FallbackChain::new("fallback-chain", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "fan-out-grid" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::fan_out_grid::FanOutGrid::new("fan-out-grid", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "flow-builder" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::flow_builder::FlowBuilder::new("flow-builder", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "handoff-swimlanes" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::handoff_swimlanes::HandoffSwimlanes::new(
                            "handoff-swimlanes",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "inline-diff" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::inline_diff::InlineDiff::new("inline-diff", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "judge-bracket" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::judge_bracket::JudgeBracket::new("judge-bracket", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "memory-graph" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::memory_graph::MemoryGraph::new("memory-graph", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "memory-panel" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::memory_panel::MemoryPanel::new("memory-panel", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "model-router" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::model_router::ModelRouter::new("model-router", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "parallel-attempts" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::parallel_attempts::ParallelAttempts::new(
                            "parallel-attempts",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "permission-request" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::permission_request::PermissionRequest::new(
                            "permission-request",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "plan-vs-actual" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::plan_vs_actual::PlanVsActual::new("plan-vs-actual", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "rate-limit-planner" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::rate_limit_planner::RateLimitPlanner::new(
                            "rate-limit-planner",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "reasoning-summary" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::reasoning_summary::ReasoningSummary::new(
                            "reasoning-summary",
                            cx,
                        )
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "repo-map" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::repo_map::RepoMap::new("repo-map", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "response-stream" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::response_stream::ResponseStream::new("response-stream", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "review-center" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::review_center::ReviewCenter::new("review-center", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "root-cause-tree" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::root_cause_tree::RootCauseTree::new("root-cause-tree", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "run-controls" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::run_controls::RunControls::new("run-controls", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "running-now" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| kui::components::running_now::RunningNow::new("running-now", cx))
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "scheduled-runs" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::scheduled_runs::ScheduledRuns::new("scheduled-runs", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "tool-access-map" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::tool_access_map::ToolAccessMap::new("tool-access-map", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "trace-waterfall" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::trace_waterfall::TraceWaterfall::new("trace-waterfall", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        "voice-session" => Some(
            cx.new(|cx| {
                let view: AnyView = cx
                    .new(|cx| {
                        kui::components::voice_session::VoiceSession::new("voice-session", cx)
                    })
                    .into();
                AffectProStage::new(view, false, cx)
            })
            .into(),
        ),
        _ => None,
    }
}
