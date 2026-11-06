//! A simulated workflow. No external services are contacted.
use gpui::{prelude::*, *};
use kui::{
    components::ai_workflow::{
        AiWorkflow, WorkflowEvent, WorkflowKind, WorkflowPath, WorkflowResult, WorkflowStep,
    },
    primitives::{control::UiLanguage, family::Paint},
};
use std::{cell::Cell, rc::Rc, time::Duration};
pub const IDS: &[&str] = &["ai-workflow"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    workflow: Entity<AiWorkflow>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let steps = vec![
            WorkflowStep::new(
                "incoming",
                t("Receive request", "收到请求"),
                t("Inbox", "收件箱"),
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
                t("Messages", "消息"),
                WorkflowKind::Action,
            )
            .with_path("urgent"),
            WorkflowStep::new(
                "task",
                t("Create a task", "创建任务"),
                t("Tasks", "任务"),
                WorkflowKind::Action,
            )
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
            WorkflowStep::new(
                "note",
                t("Add a note", "添加备注"),
                t("Notes", "笔记"),
                WorkflowKind::Action,
            ),
            WorkflowStep::new(
                "update",
                t("Update a task", "更新任务"),
                t("Tasks", "任务"),
                WorkflowKind::Action,
            ),
        ];
        let workflow = cx.new(|_| {
            AiWorkflow::new(
                t("Triage incoming requests", "处理新收到的请求"),
                t(
                    "A short trigger, AI step and two action paths",
                    "触发器、AI 步骤和两条操作分支",
                ),
                steps,
                paths,
            )
            .with_catalog(catalog)
            .with_simulated(true)
        });
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        workflow.update(cx, |workflow, cx| workflow.set_visible(visible, cx));
        let weak = workflow.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                if weak
                    .update(cx, |workflow, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        workflow.set_visible(visible, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let failed = Rc::new(Cell::new(false));
        cx.subscribe(&workflow, move |_, workflow, event: &WorkflowEvent, cx| {
            let WorkflowEvent::RunStep {
                request_id, step, ..
            } = event
            else {
                return;
            };
            let id = *request_id;
            let step_id = step.id.clone();
            let fail = step_id.as_ref() == "task" && !failed.replace(true);
            let weak = workflow.downgrade();
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(420))
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
                        "incoming" => lang.text("Sample request received", "已收到样例请求"),
                        "classify" => lang.text("Urgent · account access", "紧急 · 账户访问"),
                        "route" => lang.text("Selected urgent path", "已选择紧急分支"),
                        "notify" => lang.text("Sample notification prepared", "已准备样例通知"),
                        "task" => lang.text("Sample task prepared", "已准备样例任务"),
                        "archive" => lang.text("Sample saved for review", "样例已保存待审核"),
                        _ => lang.text("Sample action completed", "样例操作已完成"),
                    };
                    let result = WorkflowResult::new(output).with_duration(420);
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
        Self { workflow }
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
            .justify_center()
            .p(px(20.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(self.workflow.clone())
    }
}
