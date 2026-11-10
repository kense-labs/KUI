//! A simulated workflow. No external services are contacted.
use gpui::{prelude::*, *};
use kui::{
    components::ai_workflow::{
        AiWorkflow, WorkflowEvent, WorkflowKind, WorkflowPath, WorkflowResult, WorkflowStep,
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, disabled_motion},
        ops_surface::{btn, row},
    },
};
use std::{cell::Cell, rc::Rc, time::Duration};
pub const IDS: &[&str] = &["ai-workflow"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    workflow: Entity<AiWorkflow>,
    steps: Vec<WorkflowStep>,
    scenario: Rc<Cell<usize>>,
    failed: Rc<Cell<bool>>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let steps = vec![
            WorkflowStep::new(
                "incoming",
                t("Receive request", "收到请求"),
                "Gmail",
                WorkflowKind::Trigger,
            )
            .with_detail(t("Watch the support inbox", "监听支持收件箱")),
            WorkflowStep::new(
                "classify",
                t("Classify request", "分类请求"),
                "Kense AI",
                WorkflowKind::Ai,
            )
            .with_detail(t("Identify urgency and topic", "识别紧急程度和主题")),
            WorkflowStep::new(
                "route",
                t("Route request", "分流请求"),
                t("Condition", "条件"),
                WorkflowKind::Branch,
            )
            .with_detail(t(
                "Choose a path from the classification",
                "根据分类选择分支",
            )),
            WorkflowStep::new(
                "notify",
                t("Notify the team", "通知团队"),
                "Slack",
                WorkflowKind::Action,
            )
            .with_detail(t("Send to #support-urgent", "发送到紧急支持频道"))
            .with_path("urgent"),
            WorkflowStep::new(
                "task",
                t("Create a task", "创建任务"),
                "Linear",
                WorkflowKind::Action,
            )
            .with_detail(t("Create in Support", "在支持项目中创建"))
            .with_path("urgent"),
            WorkflowStep::new(
                "archive",
                t("Save for review", "保存待审核"),
                t("Inbox", "收件箱"),
                WorkflowKind::Action,
            )
            .with_path("other"),
        ];
        let paths = vec![
            WorkflowPath::new("urgent", t("Urgent", "紧急")),
            WorkflowPath::new("other", t("Other", "其他")),
        ];
        let catalog = vec![
            steps[0].clone(),
            WorkflowStep::new(
                "schedule",
                t("On a schedule", "定时触发"),
                "KUI",
                WorkflowKind::Trigger,
            ),
            WorkflowStep::new(
                "webhook",
                t("Receive a webhook", "接收 Webhook"),
                "HTTP",
                WorkflowKind::Trigger,
            ),
            WorkflowStep::new(
                "note",
                t("Add a note", "添加备注"),
                "Notion",
                WorkflowKind::Action,
            ),
            WorkflowStep::new(
                "update",
                t("Update a task", "更新任务"),
                "Linear",
                WorkflowKind::Action,
            ),
            WorkflowStep::new(
                "reply",
                t("Draft a reply", "拟定回复"),
                "Kense",
                WorkflowKind::Ai,
            ),
        ];
        let workflow = cx.new(|_| {
            AiWorkflow::new(
                t("Triage incoming requests", "处理新收到的请求"),
                t(
                    "A short trigger, AI step and two action paths",
                    "触发器、AI 步骤和两条操作分支",
                ),
                steps.clone(),
                paths,
            )
            .with_catalog(catalog)
            .with_simulated(true)
        });
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        workflow.update(cx, |workflow, cx| workflow.set_visible(visible, cx));
        let failed = Rc::new(Cell::new(false));
        let scenario = Rc::new(Cell::new(0));
        let mode = scenario.clone();
        let failure = failed.clone();
        cx.subscribe(&workflow, move |_, workflow, event: &WorkflowEvent, cx| {
            let WorkflowEvent::RunStep {
                request_id, step, ..
            } = event
            else {
                return;
            };
            let id = *request_id;
            let step_id = step.id.clone();
            let fail = mode.get() == 1 && step_id.as_ref() == "task" && !failure.replace(true);
            let duration = match step_id.as_ref() {
                "incoming" => 386,
                "classify" => 1126,
                "route" => 280,
                "notify" => 571,
                "task" => 746,
                _ => 395,
            };
            let delay = if disabled_motion(cx) { 0 } else { duration };
            let weak = workflow.downgrade();
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(delay))
                    .await;
                let _ = weak.update(cx, |workflow, cx| {
                    if fail {
                        workflow.resolve(
                            id,
                            Err(lang
                                .text(
                                    "Simulated failure. Retry to continue.",
                                    "模拟失败，请重试继续。",
                                )
                                .into()),
                            cx,
                        );
                        return;
                    }
                    let output = match step_id.as_ref() {
                        "incoming" => lang.text("Login blocked after team migration\nNora Chen · support@example.test\nOur teammates cannot sign in since we moved to the new workspace.", "团队迁移后无法登录\nNora Chen · support@example.test\n迁入新工作区后，团队成员无法登录。"),
                        "classify" => lang.text("Urgent · Account access · 96% confidence\nMultiple teammates are blocked from entering the workspace.", "紧急 · 账户访问 · 置信度 96%\n多名团队成员无法进入工作区。"),
                        "route" => "",
                        "notify" => lang.text("#support-urgent\nLogin is blocked for several teammates. Please check the migration.", "#support-urgent\n多名成员无法登录，请检查工作区迁移。"),
                        "task" => lang.text("SUP-241 · Investigate sign-in after migration\nPriority: Urgent · Support", "SUP-241 · 调查迁移后的登录问题\n优先级：紧急 · 支持"),
                        "archive" => lang.text("Saved for the next support review", "已保存至下一次支持审核"),
                        _ => lang.text("Sample action completed", "样例操作已完成"),
                    };
                    let result = WorkflowResult::new(output).with_duration(duration);
                    workflow.resolve(
                        id,
                        Ok(if step_id.as_ref() == "route" {
                            result.with_path("urgent")
                        } else {
                            result
                        }),
                        cx,
                    );
                });
            })
            .detach();
        })
        .detach();
        let weak = workflow.downgrade();
        let initial_mode = scenario.clone();
        cx.spawn(async move |_, cx| {
            while initial_mode.get() == 0 {
                let visible = weak.update(cx, |workflow, cx| {
                    let visible = cx
                        .try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)());
                    workflow.set_visible(visible, cx);
                    if visible {
                        workflow.start(cx);
                    }
                    visible
                });
                if !matches!(visible, Ok(false)) {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
            }
        })
        .detach();
        Self {
            workflow,
            steps,
            scenario,
            failed,
        }
    }
}
impl Render for Demo {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .flex_col()
            .gap_3()
            .items_center()
            .justify_start()
            .p(px(16.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(
                row()
                    .justify_center()
                    .flex_wrap()
                    .gap(px(3.))
                    .p(px(4.))
                    .bg(p.panel)
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line)
                    .children(
                        [
                            ("Passes", "成功"),
                            ("Fails", "失败"),
                            ("Loading", "加载中"),
                            ("Empty", "空状态"),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (en, zh))| {
                            btn(
                                ("workflow-scenario", i),
                                UiLanguage::current(cx).text(en, zh),
                                self.scenario.get() == i,
                                cx,
                                move |s, _, cx| {
                                    s.scenario.set(i);
                                    s.failed.set(false);
                                    let steps = if i == 3 { Vec::new() } else { s.steps.clone() };
                                    s.workflow.update(cx, |workflow, cx| {
                                        workflow.set_steps(steps, cx);
                                        workflow.set_loading(i == 2, cx);
                                        if i <= 1 {
                                            workflow.start(cx);
                                        }
                                    });
                                    cx.notify();
                                },
                            )
                        }),
                    ),
            )
            .child(div().w_full().max_w(px(522.)).child(self.workflow.clone()))
    }
}
